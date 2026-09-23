mod serial;
mod ssh;
mod storage;

use std::collections::HashMap;
use std::sync::{mpsc, Arc, Mutex};
use tauri::{AppHandle, Emitter, State};

#[derive(Debug)]
pub enum ControlMsg {
    Input(Vec<u8>),
    Resize { cols: u32, rows: u32 },
    Close,
}

#[derive(Clone)]
enum SessionControl {
    Ssh(tokio::sync::mpsc::Sender<ControlMsg>),
    Serial(mpsc::Sender<ControlMsg>),
}

#[derive(Clone, Default)]
struct AppState {
    sessions: Arc<Mutex<HashMap<String, SessionControl>>>,
}

#[tauri::command]
fn list_hosts() -> Vec<storage::Host> {
    storage::load_hosts()
}

#[tauri::command]
fn save_host(host: storage::HostInput) -> Result<storage::Host, String> {
    storage::upsert_host(host)
}

#[tauri::command]
fn delete_host(id: String) -> Result<(), String> {
    storage::delete_host(&id)
}

#[tauri::command]
async fn connect(
    app: AppHandle,
    state: State<'_, AppState>,
    session_id: String,
    host_id: String,
) -> Result<(), String> {
    let host = storage::get_host(&host_id)?;
    let sessions = state.sessions.clone();

    if host.proto == "serial" {
        let (tx, rx) = mpsc::channel::<ControlMsg>();
        sessions
            .lock()
            .unwrap()
            .insert(session_id.clone(), SessionControl::Serial(tx));
        let sid = session_id.clone();
        let app2 = app.clone();
        let sessions2 = sessions.clone();
        std::thread::spawn(move || {
            if let Err(e) = serial::connect(app2.clone(), sid.clone(), host, rx) {
                let _ = app2.emit(
                    "term-exit",
                    serde_json::json!({ "sessionId": sid, "message": e }),
                );
            }
            sessions2.lock().unwrap().remove(&sid);
        });
    } else {
        let (tx, rx) = tokio::sync::mpsc::channel::<ControlMsg>(256);
        sessions
            .lock()
            .unwrap()
            .insert(session_id.clone(), SessionControl::Ssh(tx));
        let sid = session_id.clone();
        let app2 = app.clone();
        tokio::spawn(async move {
            if let Err(e) = ssh::connect(app2.clone(), sid.clone(), host, rx).await {
                let _ = app2.emit(
                    "term-exit",
                    serde_json::json!({ "sessionId": sid, "message": e }),
                );
            }
            sessions.lock().unwrap().remove(&sid);
        });
    }

    Ok(())
}

async fn send_control(state: &AppState, session_id: &str, msg: ControlMsg) -> Result<(), String> {
    let sender = state
        .sessions
        .lock()
        .unwrap()
        .get(session_id)
        .cloned()
        .ok_or("session not found")?;
    match sender {
        SessionControl::Ssh(tx) => tx.send(msg).await.map_err(|e| e.to_string()),
        SessionControl::Serial(tx) => tx.send(msg).map_err(|e| e.to_string()),
    }
}

#[tauri::command]
async fn write(state: State<'_, AppState>, session_id: String, data: Vec<u8>) -> Result<(), String> {
    send_control(state.inner(), &session_id, ControlMsg::Input(data)).await
}

#[tauri::command]
async fn resize(
    state: State<'_, AppState>,
    session_id: String,
    cols: u32,
    rows: u32,
) -> Result<(), String> {
    send_control(state.inner(), &session_id, ControlMsg::Resize { cols, rows }).await
}

#[tauri::command]
async fn disconnect(state: State<'_, AppState>, session_id: String) -> Result<(), String> {
    send_control(state.inner(), &session_id, ControlMsg::Close).await?;
    state.sessions.lock().unwrap().remove(&session_id);
    Ok(())
}

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_process::init())
        .manage(AppState::default())
        .invoke_handler(tauri::generate_handler![
            list_hosts,
            save_host,
            delete_host,
            connect,
            write,
            resize,
            disconnect
        ])
        .run(tauri::generate_context!())
        .expect("error while running Roans Terminal");
}
