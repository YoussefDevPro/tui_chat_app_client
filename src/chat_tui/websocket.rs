use chrono::Local;
use futures_util::{SinkExt, StreamExt};
use std::thread;
use tokio_tungstenite::{connect_async, tungstenite::Message as WsMessage};

use super::data::ChatMessage;

// isock when trying to host smt
//  ):

pub fn start_ws_thread(
    ws_url: String,
    token: String,
    chat_tx: std::sync::mpsc::Sender<ChatMessage>,
    mut send_rx: tokio::sync::mpsc::UnboundedReceiver<String>,
) {
    thread::spawn(move || {
        let rt = tokio::runtime::Runtime::new().expect("Failed to create runtime");
        rt.block_on(async move {
            let (ws_stream, _) = connect_async(&ws_url).await.expect("Failed to connect WS");
            let (mut ws_write, mut ws_read) = ws_stream.split();

            ws_write
                .send(WsMessage::Text(token.into()))
                .await
                .expect("Failed to send token");

            let send_fut = tokio::spawn(async move {
                while let Some(msg) = send_rx.recv().await {
                    if !msg.trim().is_empty() {
                        let _ = ws_write.send(WsMessage::Text(msg.into())).await;
                    }
                }
            });

            while let Some(msg) = ws_read.next().await {
                if let Ok(WsMessage::Text(txt)) = msg {
                    if let Ok(parsed) = serde_json::from_str::<ChatMessage>(&txt) {
                        let _ = chat_tx.send(parsed);
                    } else {
                        let _ = chat_tx.send(ChatMessage {
                            user: "system".to_string(),
                            content: txt.to_string(),
                            icon: Some("󰚩".to_string()),
                            timestamp: Some(Local::now().timestamp()),
                        });
                    }
                }
            }
            let _ = send_fut.await;
        });
    });
}
