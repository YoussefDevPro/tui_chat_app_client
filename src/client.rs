use anyhow::Result;
use chrono::Local;
use serde_json::json;
use std::fs::OpenOptions;
use std::io::Write;
use std::sync::Arc;
use std::sync::Mutex;
use tokio::io::{AsyncWriteExt, BufReader, Lines};
use tokio::net::tcp::{OwnedReadHalf, OwnedWriteHalf};

// here there is every function we use to talk with the server, it is perhaps very basic, but its
// what we're going to use, as u see, we pass a NetDebug instance to every function to log every
// exchange between the server and the client, there is some function that are unsued, but it will
// be used for the chat, until we work on it

#[derive(Clone)]
pub struct NetDebug {
    file: Arc<Mutex<std::fs::File>>,
}

impl NetDebug {
    pub fn new(filename: &str) -> Self {
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(filename)
            .expect("Could not open debug log file");
        Self {
            file: Arc::new(Mutex::new(file)),
        }
    }

    pub fn log_send(&self, msg: &str) {
        let now = Local::now();
        let mut file = self.file.lock().unwrap();
        writeln!(file, "[{}][SEND] {}", now.format("%Y-%m-%d %H:%M:%S"), msg).ok();
    }

    pub fn log_recv(&self, msg: &str) {
        let now = Local::now();
        let mut file = self.file.lock().unwrap();
        writeln!(file, "[{}][RECV] {}", now.format("%Y-%m-%d %H:%M:%S"), msg).ok();
    }
}

async fn send_action(
    write_half: &mut OwnedWriteHalf,
    socket_lines: &mut Lines<BufReader<OwnedReadHalf>>,
    action: serde_json::Value,
    net_debug: &NetDebug,
) -> Result<serde_json::Value> {
    let action_str = serde_json::to_string(&action)?;
    net_debug.log_send(&action_str);
    write_half.write_all(action_str.as_bytes()).await?;
    write_half.write_all(b"\n").await?;
    if let Some(resp) = socket_lines.next_line().await? {
        net_debug.log_recv(&resp);
        Ok(serde_json::from_str(&resp)?)
    } else {
        net_debug.log_recv("[None]");
        Err(anyhow::anyhow!("No response from server"))
    }
}

pub fn server_response_message(resp: &serde_json::Value) -> Option<String> {
    if !resp
        .get("success")
        .and_then(|v| v.as_bool())
        .unwrap_or(false)
    {
        resp.get("error")
            .and_then(|e| e.as_str())
            .map(|s| s.to_string())
    } else {
        Some("successful    ".to_string())
    }
}

pub async fn register(
    username: &str,
    password: &str,
    icon: &str,
    write_half: &mut OwnedWriteHalf,
    socket_lines: &mut Lines<BufReader<OwnedReadHalf>>,
    net_debug: &NetDebug,
) -> Result<serde_json::Value> {
    let req = json!({
        "action": "register",
        "payload": {
            "username": username,
            "password": password,
            "icon": icon
        }
    });
    send_action(write_half, socket_lines, req, net_debug).await
}

pub async fn login(
    username: &str,
    password: &str,
    write_half: &mut OwnedWriteHalf,
    socket_lines: &mut Lines<BufReader<OwnedReadHalf>>,
    net_debug: &NetDebug,
) -> Result<serde_json::Value> {
    let req = json!({
        "action": "login",
        "payload": {
            "username": username,
            "password": password
        }
    });
    send_action(write_half, socket_lines, req, net_debug).await
}

pub async fn send_message(
    sender_id: &str,
    content: &str,
    write_half: &mut OwnedWriteHalf,
    net_debug: &NetDebug,
) -> Result<(), String> {
    let req = serde_json::json!({
        "action": "send_message",
        "payload": {
            "sender_id": sender_id,
            "content": content
        }
    });
    let raw = req.to_string() + "\n";
    net_debug.log_send(&raw);
    write_half
        .write_all(raw.as_bytes())
        .await
        .map_err(|e| e.to_string())?;
    Ok(())
}
pub async fn get_messages(
    limit: usize,
    write_half: &mut OwnedWriteHalf,
    socket_lines: &mut Lines<BufReader<OwnedReadHalf>>,
    net_debug: &NetDebug,
) -> Result<serde_json::Value> {
    let req = json!({
        "action": "get_messages",
        "payload": { "limit": limit }
    });
    send_action(write_half, socket_lines, req, net_debug).await
}

pub async fn ban_user(
    admin_id: &str,
    target_id: &str,
    reason: &str,
    write_half: &mut OwnedWriteHalf,
    socket_lines: &mut Lines<BufReader<OwnedReadHalf>>,
    net_debug: &NetDebug,
) -> Result<serde_json::Value> {
    let req = json!({
        "action": "ban_user",
        "payload": {
            "admin_id": admin_id,
            "target_id": target_id,
            "reason": reason
        }
    });
    send_action(write_half, socket_lines, req, net_debug).await
}

pub async fn unban_user(
    admin_id: &str,
    target_id: &str,
    write_half: &mut OwnedWriteHalf,
    socket_lines: &mut Lines<BufReader<OwnedReadHalf>>,
    net_debug: &NetDebug,
) -> Result<serde_json::Value> {
    let req = json!({
        "action": "unban_user",
        "payload": {
            "admin_id": admin_id,
            "target_id": target_id
        }
    });
    send_action(write_half, socket_lines, req, net_debug).await
}

pub async fn promote_user(
    admin_id: &str,
    target_id: &str,
    write_half: &mut OwnedWriteHalf,
    socket_lines: &mut Lines<BufReader<OwnedReadHalf>>,
    net_debug: &NetDebug,
) -> Result<serde_json::Value> {
    let req = json!({
        "action": "promote_user",
        "payload": {
            "admin_id": admin_id,
            "target_id": target_id
        }
    });
    send_action(write_half, socket_lines, req, net_debug).await
}

pub async fn change_username(
    user_id: &str,
    new_username: &str,
    write_half: &mut OwnedWriteHalf,
    socket_lines: &mut Lines<BufReader<OwnedReadHalf>>,
    net_debug: &NetDebug,
) -> Result<serde_json::Value> {
    let req = json!({
        "action": "change_username",
        "payload": {
            "user_id": user_id,
            "new_username": new_username
        }
    });
    send_action(write_half, socket_lines, req, net_debug).await
}

pub async fn change_icon(
    user_id: &str,
    new_icon: &str,
    write_half: &mut OwnedWriteHalf,
    socket_lines: &mut Lines<BufReader<OwnedReadHalf>>,
    net_debug: &NetDebug,
) -> Result<serde_json::Value> {
    let req = json!({
        "action": "change_icon",
        "payload": {
            "user_id": user_id,
            "new_icon": new_icon
        }
    });
    send_action(write_half, socket_lines, req, net_debug).await
}
