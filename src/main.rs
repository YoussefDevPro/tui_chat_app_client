mod auth_tui;
mod chat_tui;
mod client;

use crate::auth_tui::AuthMode;
use auth_tui::run_auth_tui;
use client::{get_messages, login, register, NetDebug};
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::net::TcpStream;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // now the connection PHASE
    // the user can choose to register an acc or login if account already registered, i made the
    // tui using ratatui, so bascaly, there is two mode, Register Mode and Login Mode, we change
    // this variable so the tui can know what to show, like the icon input bar and the textin the
    // submit button, then it cheks if everythings right , then it go to the chat, unfortunatly, i
    // still didn't done the chat part yet (there is the tui and it send the message to the server
    // , just forgot to add a listener so every time someone send a message it show it, an other
    // data as well), but tommoroy maybe, also, every time we send or receive smt from the server
    // it log in in the net_debug.log file just for debuging.
    let net_debug = NetDebug::new("net_debug.log");
    let register_closure = |username: &str, password: &str, icon: &str| {
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
                            Ok(())
                        } else {
                            let msg = resp
                                .get("error")
                                .and_then(|v| v.as_str())
                                .unwrap_or("unknown error")
                                .to_string();
                            Err(msg)
                        }
                    }
                    Err(e) => Err(e),
                }
            });
            tx.send(result).ok();
        });

        match rx.recv_timeout(std::time::Duration::from_secs(10)) {
            Ok(Ok(())) => Some(Ok(())),
            Ok(Err(msg)) => Some(Err(msg)),
            Err(_) => Some(Err("timeout lol".to_string())),
        }
    };

    let login_closure = |username: &str, password: &str, _icon: &str| {
        let (username, password) = (username.to_owned(), password.to_owned());
        let (tx, rx) = std::sync::mpsc::channel();
        let net_debug = net_debug.clone();

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
                            Ok(())
                        } else {
                            let msg = resp
                                .get("error")
                                .and_then(|v| v.as_str())
                                .unwrap_or("login failed")
                                .to_string();
                            Err(msg)
                        }
                    }
                    Err(e) => Err(e),
                }
            });
            tx.send(result).ok();
        });

        match rx.recv_timeout(std::time::Duration::from_secs(10)) {
            Ok(Ok(())) => Some(Ok(())),
            Ok(Err(msg)) => Some(Err(msg)),
            Err(_) => Some(Err("timeout".to_string())),
        }
    };

    let mut mode = AuthMode::Register;

    let (username, password, _icon) = loop {
        let auth = match mode {
            AuthMode::Register => run_auth_tui(AuthMode::Register, register_closure),
            AuthMode::Login => run_auth_tui(AuthMode::Login, login_closure),
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
            match mode {
                AuthMode::Register => break (auth.username, auth.password, auth.icon),
                AuthMode::Login => break (auth.username, auth.password, String::new()),
            }
        } else {
            return Ok(());
        }
    };

    // ========== FINAL LOGIN CONFIRMATION ==========
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
        // TODO: Replace with error popup in auth_tui
        println!("login fail: {err}");
        return Ok(());
    }

    // ========== CHAT PHASE ==========
    // New connection for chat
    let stream = TcpStream::connect("127.0.0.1:5000").await?;
    let (read_half, mut write_half) = stream.into_split();
    let mut socket_lines = BufReader::new(read_half).lines();

    let resp = get_messages(20, &mut write_half, &mut socket_lines, &net_debug).await?;
    let mut messages: Vec<String> = Vec::new();
    if let Some(msgs) = resp.get("data").and_then(|v| v.as_array()) {
        for msg in msgs {
            let sender = msg.get("sender").and_then(|v| v.as_str()).unwrap_or("??");
            let icon = msg.get("icon").and_then(|v| v.as_str()).unwrap_or("");
            let content = msg.get("content").and_then(|v| v.as_str()).unwrap_or("");
            messages.push(format!("{icon} {sender}: {content}"));
        }
    }

    chat_tui::run_chat_tui(messages);

    Ok(())
}
