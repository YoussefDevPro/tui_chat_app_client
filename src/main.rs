use serde_json::json;
use std::fs;
use std::io;
use std::path::Path;
use tokio::io::{stdin, AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpStream;
// well, ik it's a very, very confusing code, but bc i was  so exited i forgot to add comments,
// i was so exited today lol :3

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let db_path = "chat_server.db";
    let _guard = DbDropGuard(db_path);
    println!("=== Simple Chat Client ===");
    let mut stream = TcpStream::connect("127.0.0.1:6969").await?;
    let (reader, mut writer) = stream.split();
    let mut lines = BufReader::new(reader).lines();
    println!("Register or login? [r/l]: ");
    let mut mode = String::new();
    io::stdin().read_line(&mut mode)?;
    let mode = mode.trim();
    let (username, password, icon);
    if mode == "r" {
        println!("Username: ");
        username = read_line()?;
        println!("Password: ");
        password = read_line()?;
        println!("Icon (Nerd Font, copy/paste): ");
        icon = read_line()?;
        let register = json!({
            "action": "register",
            "payload": {
                "username": username,
                "password": password,
                "icon": icon
            }
        });
        writer
            .write_all(serde_json::to_string(&register)?.as_bytes())
            .await?;
        writer.write_all(b"\n").await?;
        let resp = lines.next_line().await?.unwrap_or_default();
        println!("Server: {resp}");
        let resp_json: serde_json::Value = serde_json::from_str(&resp)?;
        if !resp_json
            .get("success")
            .and_then(|v| v.as_bool())
            .unwrap_or(false)
        {
            println!("Registration failed. Exiting.");
            return Ok(());
        }
    } else {
        println!("Username: ");
        username = read_line()?;
        println!("Password: ");
        password = read_line()?;
        icon = "N/A".to_owned();
        let login = json!({
            "action": "login",
            "payload": {
                "username": username,
                "password": password
            }
        });
        writer
            .write_all(serde_json::to_string(&login)?.as_bytes())
            .await?;
        writer.write_all(b"\n").await?;
        let resp = lines.next_line().await?.unwrap_or_default();
        println!("Server: {resp}");
        let resp_json: serde_json::Value = serde_json::from_str(&resp)?;
        if !resp_json
            .get("success")
            .and_then(|v| v.as_bool())
            .unwrap_or(false)
        {
            println!("Login failed. Exiting.");
            return Ok(());
        }
    }

    let user_id: String = {
        let last_resp = lines.next_line().await?.unwrap_or_default();
        let resp_json: serde_json::Value = serde_json::from_str(&last_resp).unwrap_or_default();
        resp_json
            .get("data")
            .and_then(|d| d.get("user_id"))
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_owned()
    };

    println!("Logged in as: {username}");

    let mut lines2 = lines;
    tokio::spawn(async move {
        loop {
            if let Some(Ok(line)) = lines2.next_line().await {
                println!("[Server msg] {line}");
            }
        }
    });

    loop {
        println!("Type message, or /cmd:");
        let input = read_line()?;
        if input == "/quit" {
            break;
        }
        if input == "/get" {
            let get = json!({
                "action": "get_messages",
                "payload": { "limit": 10 }
            });
            writer
                .write_all(serde_json::to_string(&get)?.as_bytes())
                .await?;
            writer.write_all(b"\n").await?;
        } else if input.starts_with("/ban ") {
            let args: Vec<&str> = input["/ban ".len()..].split_whitespace().collect();
            if args.len() < 2 {
                println!("Usage: /ban <target_id> <reason>");
                continue;
            }
            let target_id = args[0];
            let reason = args[1..].join(" ");
            let ban = json!({
                "action": "ban_user",
                "payload": {
                    "admin_id": user_id,
                    "target_id": target_id,
                    "reason": reason
                }
            });
            writer
                .write_all(serde_json::to_string(&ban)?.as_bytes())
                .await?;
            writer.write_all(b"\n").await?;
        } else if input.starts_with("/unban ") {
            let target_id = input["/unban ".len()..].trim();
            let unban = json!({
                "action": "unban_user",
                "payload": {
                    "admin_id": user_id,
                    "target_id": target_id
                }
            });
            writer
                .write_all(serde_json::to_string(&unban)?.as_bytes())
                .await?;
            writer.write_all(b"\n").await?;
        } else if input.starts_with("/promote ") {
            let target_id = input["/promote ".len()..].trim();
            let promote = json!({
                "action": "promote_user",
                "payload": {
                    "admin_id": user_id,
                    "target_id": target_id
                }
            });
            writer
                .write_all(serde_json::to_string(&promote)?.as_bytes())
                .await?;
            writer.write_all(b"\n").await?;
        } else if input.starts_with("/rename ") {
            let new_username = input["/rename ".len()..].trim();
            let rename = json!({
                "action": "change_username",
                "payload": {
                    "user_id": user_id,
                    "new_username": new_username
                }
            });
            writer
                .write_all(serde_json::to_string(&rename)?.as_bytes())
                .await?;
            writer.write_all(b"\n").await?;
        } else if input.starts_with("/icon ") {
            let new_icon = input["/icon ".len()..].trim();
            let icon = json!({
                "action": "change_icon",
                "payload": {
                    "user_id": user_id,
                    "new_icon": new_icon
                }
            });
            writer
                .write_all(serde_json::to_string(&icon)?.as_bytes())
                .await?;
            writer.write_all(b"\n").await?;
        } else {
            let msg = json!({
                "action": "send_message",
                "payload": {
                    "sender_id": user_id,
                    "content": input
                }
            });
            writer
                .write_all(serde_json::to_string(&msg)?.as_bytes())
                .await?;
            writer.write_all(b"\n").await?;
        }
    }

    println!("Goodbye!");
    Ok(())
}

fn read_line() -> io::Result<String> {
    let mut buf = String::new();
    io::stdin().read_line(&mut buf)?;
    Ok(buf.trim().to_string())
}

struct DbDropGuard<'a>(&'a str);
impl<'a> Drop for DbDropGuard<'a> {
    fn drop(&mut self) {
        if Path::new(self.0).exists() {
            let _ = fs::remove_file(self.0);
            println!("[DB] Removed {}", self.0);
        }
    }
}
