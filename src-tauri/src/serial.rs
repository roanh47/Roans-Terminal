use crate::storage::Host;
use crate::ControlMsg;
use std::io::{Read, Write};
use std::sync::mpsc;
use std::time::Duration;
use tauri::{AppHandle, Emitter};

pub fn connect(
    app: AppHandle,
    session_id: String,
    host: Host,
    rx: mpsc::Receiver<ControlMsg>,
) -> Result<(), String> {
    let path = host.serial_port.clone().ok_or("no serial port set")?;
    let baud = host.baud.unwrap_or(115200);

    let mut port = serialport::new(&path, baud)
        .timeout(Duration::from_millis(50))
        .open()
        .map_err(|e| format!("serial open failed: {e}"))?;

    let mut buf = [0u8; 4096];
    loop {
        match port.read(&mut buf) {
            Ok(0) => {}
            Ok(n) => {
                let data = buf[..n].to_vec();
                let _ = app.emit("term-data", serde_json::json!({ "sessionId": session_id, "data": data }));
            }
            Err(ref e) if e.kind() == std::io::ErrorKind::TimedOut => {}
            Err(e) => {
                let _ = app.emit("term-exit", serde_json::json!({ "sessionId": session_id, "message": format!("serial read error: {e}") }));
                return Ok(());
            }
        }

        while let Ok(msg) = rx.try_recv() {
            match msg {
                ControlMsg::Input(data) => {
                    let _ = port.write_all(&data);
                    let _ = port.flush();
                }
                ControlMsg::Resize { .. } => {}
                ControlMsg::Close => {
                    let _ = app.emit("term-exit", serde_json::json!({ "sessionId": session_id }));
                    return Ok(());
                }
            }
        }
    }
}
