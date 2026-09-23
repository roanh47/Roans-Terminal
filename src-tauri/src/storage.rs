use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

const SERVICE: &str = "roans-terminal";

#[derive(Serialize, Deserialize, Clone)]
pub struct Host {
    pub id: String,
    pub name: String,
    pub proto: String,
    pub host: Option<String>,
    pub port: Option<u16>,
    pub username: Option<String>,
    pub auth: Option<String>,
    pub key_path: Option<String>,
    pub has_password: bool,
    pub has_key: bool,
    pub serial_port: Option<String>,
    pub baud: Option<u32>,
}

#[derive(Deserialize)]
pub struct HostInput {
    pub id: Option<String>,
    pub name: String,
    pub proto: String,
    pub host: Option<String>,
    pub port: Option<u16>,
    pub username: Option<String>,
    pub auth: Option<String>,
    pub password: Option<String>,
    pub key_path: Option<String>,
    pub passphrase: Option<String>,
    pub serial_port: Option<String>,
    pub baud: Option<u32>,
}

fn config_dir() -> PathBuf {
    let d = dirs::config_dir().unwrap_or_else(|| PathBuf::from(".")).join("roans-terminal");
    let _ = fs::create_dir_all(&d);
    d
}

fn hosts_file() -> PathBuf {
    config_dir().join("hosts.json")
}

pub fn load_hosts() -> Vec<Host> {
    match fs::read_to_string(hosts_file()) {
        Ok(s) => serde_json::from_str(&s).unwrap_or_default(),
        Err(_) => Vec::new(),
    }
}

fn persist(hosts: &[Host]) -> Result<(), String> {
    let data = serde_json::to_string_pretty(hosts).map_err(|e| e.to_string())?;
    fs::write(hosts_file(), data).map_err(|e| e.to_string())
}

pub fn get_host(id: &str) -> Result<Host, String> {
    load_hosts()
        .into_iter()
        .find(|h| h.id == id)
        .ok_or_else(|| "host not found".to_string())
}

fn set_secret(key: &str, value: &str) -> Result<(), String> {
    keyring::Entry::new(SERVICE, key)
        .map_err(|e| e.to_string())?
        .set_password(value)
        .map_err(|e| e.to_string())
}

fn delete_secret(key: &str) {
    if let Ok(entry) = keyring::Entry::new(SERVICE, key) {
        let _ = entry.delete_credential();
    }
}

pub fn get_secret(key: &str) -> Option<String> {
    keyring::Entry::new(SERVICE, key)
        .ok()
        .and_then(|e| e.get_password().ok())
}

pub fn upsert_host(input: HostInput) -> Result<Host, String> {
    let mut hosts = load_hosts();
    let id = input.id.clone().unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
    let auth = input.auth.clone().unwrap_or_else(|| "password".to_string());

    let pw_key = format!("{id}:password");
    let pp_key = format!("{id}:passphrase");

    let has_password = match input.password.as_deref() {
        Some(p) if !p.is_empty() => {
            set_secret(&pw_key, p)?;
            true
        }
        _ => get_secret(&pw_key).is_some(),
    };

    let has_key = match input.passphrase.as_deref() {
        Some(p) if !p.is_empty() => {
            set_secret(&pp_key, p)?;
            true
        }
        _ => get_secret(&pp_key).is_some(),
    };

    let host = Host {
        id: id.clone(),
        name: input.name,
        proto: input.proto,
        host: input.host,
        port: input.port,
        username: input.username,
        auth: Some(auth),
        key_path: input.key_path,
        has_password,
        has_key,
        serial_port: input.serial_port,
        baud: input.baud,
    };

    if let Some(existing) = hosts.iter_mut().find(|h| h.id == id) {
        *existing = host.clone();
    } else {
        hosts.push(host.clone());
    }
    persist(&hosts)?;
    Ok(host)
}

pub fn delete_host(id: &str) -> Result<(), String> {
    let mut hosts = load_hosts();
    hosts.retain(|h| h.id != id);
    persist(&hosts)?;
    delete_secret(&format!("{id}:password"));
    delete_secret(&format!("{id}:passphrase"));
    Ok(())
}
