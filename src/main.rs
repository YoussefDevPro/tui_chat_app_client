use serde_json::json;
use std::io;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpStream;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("=== Simple Chat Client ===");

    let stream = TcpStream::connect("127.0.0.1:5000").await?;
    let (read_half, mut write_half) = stream.into_split();
    let mut socket_lines = BufReader::new(read_half).lines();
    let mut stdin_lines = BufReader::new(tokio::io::stdin()).lines();

    // Registration or login
    println!("Register or login? [r/l]: ");
    let mode = stdin_lines.next_line().await?.unwrap_or_default();
    let (username, password, icon, mut user_id);

    if mode.trim() == "r" {
        println!("Username: ");
        username = stdin_lines.next_line().await?.unwrap_or_default();
        println!("Password: ");
        password = stdin_lines.next_line().await?.unwrap_or_default();
        println!("Icon (Nerd Font, copy/paste): ");
        icon = stdin_lines.next_line().await?.unwrap_or_default();

        let register = json!({
            "action": "register",
            "payload": {
                "username": username,
                "password": password,
                "icon": icon
            }
        });
        write_half
            .write_all(serde_json::to_string(&register)?.as_bytes())
            .await?;
        write_half.write_all(b"\n").await?;
    } else {
        println!("Username: ");
        username = stdin_lines.next_line().await?.unwrap_or_default();
        println!("Password: ");
        password = stdin_lines.next_line().await?.unwrap_or_default();
        icon = "".to_owned();

        let login = json!({
            "action": "login",
            "payload": {
                "username": username,
                "password": password
            }
        });
        write_half
            .write_all(serde_json::to_string(&login)?.as_bytes())
            .await?;
        write_half.write_all(b"\n").await?;
    }

    // Read the server response and extract user_id
    user_id = String::new();
    if let Some(resp) = socket_lines.next_line().await? {
        println!("Server: {resp}");
        let resp_json: serde_json::Value = serde_json::from_str(&resp).unwrap_or_default();
        user_id = resp_json
            .get("data")
            .and_then(|d| d.get("user_id"))
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        if user_id.is_empty() {
            println!("Failed to get user_id. Exiting.");
            return Ok(());
        }
        println!("Logged in as: {username}");
    }

    println!("Type a message, or use /get /ban /unban /promote /rename /icon /quit");

    loop {
        print!("> ");
        io::Write::flush(&mut io::stdout())?;
        let input = stdin_lines.next_line().await?.unwrap_or_default();
        if input == "/quit" {
            break;
        }
        if input == "/get" {
            let get = json!({
                "action": "get_messages",
                "payload": { "limit": 10 }
            });
            write_half
                .write_all(serde_json::to_string(&get)?.as_bytes())
                .await?;
            write_half.write_all(b"\n").await?;
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
            write_half
                .write_all(serde_json::to_string(&ban)?.as_bytes())
                .await?;
            write_half.write_all(b"\n").await?;
        } else if input.starts_with("/unban ") {
            let target_id = input["/unban ".len()..].trim();
            let unban = json!({
                "action": "unban_user",
                "payload": {
                    "admin_id": user_id,
                    "target_id": target_id
                }
            });
            write_half
                .write_all(serde_json::to_string(&unban)?.as_bytes())
                .await?;
            write_half.write_all(b"\n").await?;
        } else if input.starts_with("/promote ") {
            let target_id = input["/promote ".len()..].trim();
            let promote = json!({
                "action": "promote_user",
                "payload": {
                    "admin_id": user_id,
                    "target_id": target_id
                }
            });
            write_half
                .write_all(serde_json::to_string(&promote)?.as_bytes())
                .await?;
            write_half.write_all(b"\n").await?;
        } else if input.starts_with("/rename ") {
            let new_username = input["/rename ".len()..].trim();
            let rename = json!({
                "action": "change_username",
                "payload": {
                    "user_id": user_id,
                    "new_username": new_username
                }
            });
            write_half
                .write_all(serde_json::to_string(&rename)?.as_bytes())
                .await?;
            write_half.write_all(b"\n").await?;
        } else if input.starts_with("/icon ") {
            let new_icon = input["/icon ".len()..].trim();
            let icon = json!({
                "action": "change_icon",
                "payload": {
                    "user_id": user_id,
                    "new_icon": new_icon
                }
            });
            write_half
                .write_all(serde_json::to_string(&icon)?.as_bytes())
                .await?;
            write_half.write_all(b"\n").await?;
        } else {
            let msg = json!({
                "action": "send_message",
                "payload": {
                    "sender_id": user_id,
                    "content": input
                }
            });
            write_half
                .write_all(serde_json::to_string(&msg)?.as_bytes())
                .await?;
            write_half.write_all(b"\n").await?;
        }

        // Print any message or response from the server
        if let Some(line) = socket_lines.next_line().await? {
            println!("[Server] {line}");
        }
    }

    println!("Goodbye!");
    Ok(())
}
