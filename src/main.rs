mod auth_tui;
mod chat_tui;
mod client;

use crate::auth_tui::AuthMode;
use auth_tui::run_auth_tui;
use client::{login, register, NetDebug};
use std::sync::Arc;
use std::sync::Mutex;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::net::TcpStream;

const MESSAGE_LIMIT: usize = 50;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // ========== NET DEBUG ==========
    let net_debug = NetDebug::new("net_debug.log");

    // ========== AUTH PHASE ==========

    // These closures only check if the credentials are accepted by the server (no user_id extraction here)
    let mut register_closure =
        |username: &str, password: &str, icon: &str| -> Option<Result<(), String>> {
            let net_debug = net_debug.clone();
            let (username, password, icon) =
                (username.to_owned(), password.to_owned(), icon.to_owned());
            let (tx, rx) = std::sync::mpsc::channel();

            std::thread::spawn(move || {
                let rt = tokio::runtime::Runtime::new().unwrap();
                let result = rt.block_on(async move {
                    let stream = TcpStream::connect("127.0.0.1:5000")
                        .await
                        .map_err(|e| e.to_string())?;
                    let (read_half, mut write_half) = stream.into_split();
                    let mut socket_lines = BufReader::new(read_half).lines();

                    match register(
                        &username,
                        &password,
                        &icon,
                        &mut write_half,
                        &mut socket_lines,
                        &net_debug,
                    )
                    .await
                    .map_err(|e| e.to_string())
                    {
                        Ok(resp) => {
                            if resp.get("success").and_then(|v| v.as_bool()) == Some(true) {
                                tx.send(true).ok();
                                Ok(())
                            } else {
                                let msg = resp
                                    .get("error")
                                    .and_then(|v| v.as_str())
                                    .unwrap_or("unknown error")
                                    .to_string();
                                tx.send(false).ok();
                                Err(msg)
                            }
                        }
                        Err(e) => {
                            tx.send(false).ok();
                            Err(e)
                        }
                    }
                });
                if let Err(e) = result {
                    eprintln!("Registration failed: {e}");
                }
            });

            match rx.recv_timeout(std::time::Duration::from_secs(10)) {
                Ok(true) => Some(Ok(())),
                Ok(false) => Some(Err("Registration failed".to_string())),
                Err(_) => Some(Err("timeout lol".to_string())),
            }
        };

    let mut login_closure =
        |username: &str, password: &str, _icon: &str| -> Option<Result<(), String>> {
            let (username, password) = (username.to_owned(), password.to_owned());
            let net_debug = net_debug.clone();
            let (tx, rx) = std::sync::mpsc::channel();

            std::thread::spawn(move || {
                let rt = tokio::runtime::Runtime::new().unwrap();
                let result = rt.block_on(async move {
                    let stream = TcpStream::connect("127.0.0.1:5000")
                        .await
                        .map_err(|e| e.to_string())?;
                    let (read_half, mut write_half) = stream.into_split();
                    let mut socket_lines = BufReader::new(read_half).lines();

                    match login(
                        &username,
                        &password,
                        &mut write_half,
                        &mut socket_lines,
                        &net_debug,
                    )
                    .await
                    .map_err(|e| e.to_string())
                    {
                        Ok(resp) => {
                            if resp.get("success").and_then(|v| v.as_bool()) == Some(true) {
                                tx.send(true).ok();
                                Ok(())
                            } else {
                                let msg = resp
                                    .get("error")
                                    .and_then(|v| v.as_str())
                                    .unwrap_or("login failed")
                                    .to_string();
                                tx.send(false).ok();
                                Err(msg)
                            }
                        }
                        Err(e) => {
                            tx.send(false).ok();
                            Err(e)
                        }
                    }
                });
                if let Err(e) = result {
                    eprintln!("Login failed: {e}");
                }
            });

            match rx.recv_timeout(std::time::Duration::from_secs(10)) {
                Ok(true) => Some(Ok(())),
                Ok(false) => Some(Err("Login failed".to_string())),
                Err(_) => Some(Err("timeout lol".to_string())),
            }
        };

    let mut mode = AuthMode::Register;

    // ========== AUTH TUI LOOP ==========

    let (username, password, icon) = loop {
        let auth = match mode {
            AuthMode::Register => run_auth_tui(AuthMode::Register, &mut register_closure),
            AuthMode::Login => run_auth_tui(AuthMode::Login, &mut login_closure),
        };

        if auth.switch_to_login {
            mode = AuthMode::Login;
            continue;
        }
        if auth.switch_to_register {
            mode = AuthMode::Register;
            continue;
        }
        if auth.done {
            break (auth.username, auth.password, auth.icon);
        } else {
            return Ok(());
        }
    };

    // ========== FINAL LOGIN and USER_ID EXTRACTION ==========

    let stream = TcpStream::connect("127.0.0.1:5000").await?;
    let (read_half, mut write_half) = stream.into_split();
    let mut socket_lines = BufReader::new(read_half).lines();
    let resp = login(
        &username,
        &password,
        &mut write_half,
        &mut socket_lines,
        &net_debug,
    )
    .await?;

    if resp.get("success").and_then(|v| v.as_bool()) != Some(true) {
        let err = resp
            .get("error")
            .and_then(|v| v.as_str())
            .unwrap_or("login sus");
        println!("login fail: {err}");
        return Ok(());
    }

    let user_id = resp
        .get("data")
        .and_then(|d| d.get("user_id"))
        .and_then(|u| u.as_str())
        .map(|s| s.to_string())
        .expect("user_id must be set");

    // ========== CHAT PHASE ==========

    let net_debug = NetDebug::new("net_debug.log");
    let stream = TcpStream::connect("127.0.0.1:5000").await?;
    let (read_half, mut write_half) = stream.into_split();
    let reader = BufReader::new(read_half);
    let mut lines = reader.lines();

    // Get last N messages for initial state
    let resp = client::get_messages(MESSAGE_LIMIT, &mut write_half, &mut lines, &net_debug).await?;

    let mut messages: Vec<String> = Vec::new();
    if let Some(msgs) = resp.get("data").and_then(|v| v.as_array()) {
        for msg in msgs {
            let sender = msg.get("sender").and_then(|v| v.as_str()).unwrap_or("??");
            let icon = msg.get("icon").and_then(|v| v.as_str()).unwrap_or("");
            let content = msg.get("content").and_then(|v| v.as_str()).unwrap_or("");
            messages.push(format!("{icon} {sender}: {content}"));
        }
    }

    // Shared message buffer for TUI and receiver
    let messages = Arc::new(Mutex::new(messages));
    let messages_for_rx = messages.clone();
    let net_debug_rx = net_debug.clone();

    // Spawn a Tokio task to receive server messages in real time
    tokio::spawn(async move {
        let mut lines = lines;
        while let Ok(Some(line)) = lines.next_line().await {
            net_debug_rx.log_recv(&line);

            if let Ok(resp) = serde_json::from_str::<serde_json::Value>(&line) {
                if let Some(data) = resp.get("data") {
                    if let Some(content) = data.get("content").and_then(|v| v.as_str()) {
                        let sender = data.get("sender").and_then(|v| v.as_str()).unwrap_or("??");
                        let icon = data.get("icon").and_then(|v| v.as_str()).unwrap_or("");
                        let msg = format!("{icon} {sender}: {content}");
                        println!("Received: {msg}");

                        let mut msgs = messages_for_rx.lock().unwrap();
                        msgs.push(msg);
                        let len = msgs.len();
                        if len > MESSAGE_LIMIT {
                            msgs.drain(0..(len - MESSAGE_LIMIT));
                        }
                    }
                }
            }
        }
    });

    // Now run the TUI, which reads from the shared message buffer
    chat_tui::run_chat_tui(write_half, messages, user_id, net_debug).await;

    Ok(())
}
