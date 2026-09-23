use crate::storage::{self, Host};
use crate::ControlMsg;
use russh::client;
use russh::keys::*;
use russh::*;
use std::sync::Arc;
use std::time::Duration;
use tauri::{AppHandle, Emitter};

#[derive(Clone)]
pub struct Client;

impl client::Handler for Client {
    type Error = russh::Error;

    async fn check_server_key(
        &mut self,
        _server_public_key: &PublicKeyOrCertificate,
    ) -> Result<bool, Self::Error> {
        // TODO: persist + verify host keys. First-run trust-all for now.
        Ok(true)
    }
}

pub async fn connect(
    app: AppHandle,
    session_id: String,
    host: Host,
    mut rx: tokio::sync::mpsc::Receiver<ControlMsg>,
) -> Result<(), String> {
    let addr = host.host.clone().unwrap_or_default();
    let port = host.port.unwrap_or(22);
    let user = host.username.clone().unwrap_or_default();

    let config = Arc::new(client::Config {
        inactivity_timeout: Some(Duration::from_secs(120)),
        ..Default::default()
    });

    let mut session = client::connect(config, (addr.as_str(), port), Client)
        .await
        .map_err(|e| format!("connect failed: {e}"))?;

    // Authenticate
    let auth = host.auth.clone().unwrap_or_else(|| "password".to_string());
    if auth == "key" {
        let key_path = host.key_path.clone().ok_or("no private key path set")?;
        let passphrase = storage::get_secret(&format!("{}:passphrase", host.id));
        let key = load_secret_key(key_path, passphrase.as_deref())
            .map_err(|e| format!("failed to load private key: {e}"))?;
        let hash = session
            .best_supported_rsa_hash()
            .await
            .map_err(|e| e.to_string())?
            .flatten();
        let res = session
            .authenticate_publickey(user, PrivateKeyWithHashAlg::new(Arc::new(key), hash))
            .await
            .map_err(|e| format!("auth failed: {e}"))?;
        if !res.success() {
            return Err("public key authentication failed".to_string());
        }
    } else {
        let password = storage::get_secret(&format!("{}:password", host.id))
            .ok_or("no password stored for this host")?;
        let res = session
            .authenticate_password(user, password)
            .await
            .map_err(|e| format!("auth failed: {e}"))?;
        if !res.success() {
            return Err("password authentication failed".to_string());
        }
    }

    let mut channel = session
        .channel_open_session()
        .await
        .map_err(|e| e.to_string())?;
    channel
        .request_pty(false, "xterm-256color", 80, 24, 0, 0, &[])
        .await
        .map_err(|e| e.to_string())?;
    channel
        .request_shell(true)
        .await
        .map_err(|e| e.to_string())?;

    loop {
        tokio::select! {
            msg = channel.wait() => {
                match msg {
                    Some(ChannelMsg::Data { data }) => {
                        let bytes: Vec<u8> = data.as_ref().to_vec();
                        let _ = app.emit("term-data", serde_json::json!({ "sessionId": session_id, "data": bytes }));
                    }
                    Some(ChannelMsg::ExitStatus { .. })
                    | Some(ChannelMsg::Close)
                    | Some(ChannelMsg::Eof)
                    | None => {
                        break;
                    }
                    _ => {}
                }
            }
            control = rx.recv() => {
                match control {
                    Some(ControlMsg::Input(data)) => {
                        if channel.data(&data[..]).await.is_err() { break; }
                    }
                    Some(ControlMsg::Resize { cols, rows }) => {
                        let _ = channel.window_change(cols, rows, 0, 0).await;
                    }
                    Some(ControlMsg::Close) | None => {
                        let _ = session.disconnect(Disconnect::ByApplication, "", "English").await;
                        break;
                    }
                }
            }
        }
    }

    let _ = app.emit("term-exit", serde_json::json!({ "sessionId": session_id }));
    Ok(())
}
