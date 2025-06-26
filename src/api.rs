use futures_util::{SinkExt, StreamExt};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use tokio::net::TcpStream;
use tokio_tungstenite::{
    connect_async, tungstenite::Message as WsMessage, MaybeTlsStream, WebSocketStream,
};
use tungstenite::Message;

#[derive(Serialize)]
pub struct RegisterInput {
    pub username: String,
    pub password_hash: String,
}

#[derive(Serialize)]
pub struct LoginInput {
    pub username: String,
    pub password_hash: String,
}

#[derive(Deserialize, Debug, Clone)]
pub struct TokenResponse {
    pub token: String,
}

#[derive(Deserialize, Debug, Clone)]
pub struct ChatMessage {
    pub user: String,
    pub content: String,
    pub timestamp: i64,
}

/// Try to register. If user exists, log in. Returns JWT token on success.
pub async fn login_or_register(
    username: &str,
    password: &str,
    api_base: &str,
) -> Result<String, String> {
    let client = Client::new();

    // Register
    let register_url = format!("{}/auth/register", api_base);
    let register_body = RegisterInput {
        username: username.to_string(),
        password_hash: password.to_string(),
    };

    let register_res = client
        .post(&register_url)
        .json(&register_body)
        .send()
        .await
        .map_err(|e| format!("Register request failed: {e}"))?;

    if register_res.status().is_success() {
        // Registration succeeded, parse token
        let tok: TokenResponse = register_res
            .json()
            .await
            .map_err(|e| format!("Invalid registration response: {e}"))?;
        return Ok(tok.token);
    } else if register_res.status() == reqwest::StatusCode::CONFLICT {
        // Already registered, try login
        let login_url = format!("{}/auth/login", api_base);
        let login_body = LoginInput {
            username: username.to_string(),
            password_hash: password.to_string(),
        };
        let login_res = client
            .post(&login_url)
            .json(&login_body)
            .send()
            .await
            .map_err(|e| format!("Login request failed: {e}"))?;

        if login_res.status().is_success() {
            let tok: TokenResponse = login_res
                .json()
                .await
                .map_err(|e| format!("Invalid login response: {e}"))?;
            return Ok(tok.token);
        } else {
            let err = login_res.text().await.unwrap_or_default();
            return Err(format!("Login failed: {}", err));
        }
    } else {
        let err = register_res.text().await.unwrap_or_default();
        return Err(format!("Register failed: {}", err));
    }
}

/// Connect to WebSocket and send JWT as first message.
/// Returns the WebSocket stream split into sender and receiver.
pub async fn connect_chat_ws(
    ws_url: &str,
    jwt: &str,
) -> Result<
    (
        futures_util::stream::SplitSink<WebSocketStream<MaybeTlsStream<TcpStream>>, WsMessage>,
        futures_util::stream::SplitStream<WebSocketStream<MaybeTlsStream<TcpStream>>>,
    ),
    String,
> {
    let (ws_stream, _) = connect_async(ws_url)
        .await
        .map_err(|e| format!("WebSocket connect failed: {e}"))?;

    let (mut write, read) = ws_stream.split();

    // Send JWT as first message (plain text)
    write
        .send(WsMessage::Text(jwt.to_string().into()))
        .await
        .map_err(|e| format!("Failed to send JWT: {e}"))?;

    Ok((write, read))
}

/// Send a chat message (just the content) to the WebSocket
pub async fn send_chat_message<S>(ws_write: &mut S, content: &str) -> Result<(), String>
where
    S: SinkExt<WsMessage> + Unpin,
    <S as futures_util::Sink<Message>>::Error: std::fmt::Display,
{
    ws_write
        .send(WsMessage::Text(content.to_string().into()))
        .await
        .map_err(|e| format!("Send failed: {}", e))
}

/// Wait for a chat message from the WebSocket (blocking until next message).
/// Returns the parsed ChatMessage struct if the server sends messages as JSON.
pub async fn recv_chat_message<S>(ws_read: &mut S) -> Result<ChatMessage, String>
where
    S: StreamExt<Item = Result<WsMessage, tokio_tungstenite::tungstenite::Error>> + Unpin,
{
    while let Some(msg) = ws_read.next().await {
        match msg {
            Ok(WsMessage::Text(text)) => {
                let chat: serde_json::Result<ChatMessage> = serde_json::from_str(&text);
                if let Ok(chat_msg) = chat {
                    return Ok(chat_msg);
                } else {
                    // If it's not a chat message, just continue
                    continue;
                }
            }
            Ok(WsMessage::Close(_)) => {
                return Err("WebSocket closed by server".into());
            }
            Ok(_) => continue,
            Err(e) => return Err(format!("Receive error: {e}")),
        }
    }
    Err("WebSocket stream ended".into())
}
