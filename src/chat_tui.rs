use crate::app::App;
use crate::mpsc::UnboundedSender;
use chrono::Local;
use crossterm::event::Event;
use crossterm::event::KeyCode;
use crossterm::event::KeyEvent;
use futures_util::{SinkExt, StreamExt};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Wrap},
    Frame,
};
use serde::Deserialize;
use std::{
    fs,
    path::PathBuf,
    sync::mpsc::{Receiver, Sender},
    thread,
};
use tokio_tungstenite::{connect_async, tungstenite::Message as WsMessage};

pub async fn handle_event(
    evt: Event,
    app: &mut App,
    tx: &UnboundedSender<String>,
    chat_rx: &Receiver<ChatMessage>,
) {
    // Process all new incoming chat messages first
    while let Ok(msg) = chat_rx.try_recv() {
        app.chat_messages.push(msg);
        // Optionally: keep the list to a fixed size
        if app.chat_messages.len() > 1000 {
            app.chat_messages.remove(0);
        }
    }

    // Now process input events
    if let Event::Key(KeyEvent { code, .. }) = evt {
        match code {
            KeyCode::Char(c) => {
                app.chat_input.push(c);
            }
            KeyCode::Backspace => {
                app.chat_input.pop();
            }
            KeyCode::Enter => {
                let msg = app.chat_input.trim();
                if !msg.is_empty() {
                    // Send the message to the async WebSocket sender
                    let _ = tx.send(msg.to_string());
                    app.chat_input.clear();
                }
            }
            // Optionally: add support for left/right navigation in input
            KeyCode::Esc => {
                // You might want to implement Esc to clear input or leave page
                // For now, do nothing
            }
            _ => {}
        }
    }
}

#[derive(Deserialize, Clone, Debug)]
pub struct Rgb(u8, u8, u8);

#[derive(Deserialize, Debug)]
pub struct Theme {
    pub border: Rgb,
    pub border_focus: Rgb,
    pub button: Rgb,
    pub button_focus: Rgb,
    pub error_bg: Rgb,
    pub error_fg: Rgb,
    pub text: Rgb,
}

fn rgb_to_color(rgb: &Rgb) -> Color {
    Color::Rgb(rgb.0, rgb.1, rgb.2)
}

fn get_theme() -> Theme {
    let theme_path = PathBuf::from("theme.json");
    let data = fs::read_to_string(theme_path).expect("theme.json not found");
    serde_json::from_str(&data).expect("Invalid theme.json")
}

fn config_dir() -> PathBuf {
    #[cfg(target_os = "windows")]
    {
        std::env::var("APPDATA")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from("."))
            .join("ReeTui")
    }
    #[cfg(not(target_os = "windows"))]
    {
        let home = std::env::var("HOME").unwrap_or_else(|_| String::from("."));
        PathBuf::from(home).join(".config/reetui")
    }
}

fn get_token_from_file() -> Option<String> {
    let mut path = config_dir();
    path.push("token");
    fs::read_to_string(path).ok()
}

#[derive(Debug, Clone, Deserialize)]
pub struct ChatMessage {
    pub user: String,
    pub content: String,
    pub timestamp: Option<String>,
}

pub fn ui(f: &mut Frame, _app: &App, chat_messages: &[ChatMessage], input_value: &str) {
    let theme = get_theme();
    let area = centered_rect(80, 80, f.area());

    let layout = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([
            Constraint::Min(2),    // Chat messages
            Constraint::Length(3), // Input bar
        ])
        .split(area);

    // Chat Messages
    let mut chat_lines = Vec::new();
    for msg in chat_messages {
        let ts = Span::styled(
            format!(
                "[{}]",
                msg.timestamp
                    .clone()
                    .unwrap_or_else(|| Local::now().format("%H:%M").to_string())
            ),
            Style::default()
                .fg(rgb_to_color(&theme.button))
                .add_modifier(Modifier::DIM),
        );
        let user = Span::styled(
            &msg.user,
            Style::default()
                .fg(rgb_to_color(&theme.button_focus))
                .add_modifier(Modifier::BOLD),
        );
        let ct = Span::styled(
            format!(": {}", &msg.content),
            Style::default().fg(rgb_to_color(&theme.text)),
        );
        chat_lines.push(Line::from(vec![ts, Span::raw(" "), user, ct]));
    }
    let chat_block = Block::default()
        .borders(Borders::ALL)
        .title("Chat")
        .border_style(Style::default().fg(rgb_to_color(&theme.border)));
    let chat_box = Paragraph::new(chat_lines)
        .block(chat_block)
        .wrap(Wrap { trim: false });
    f.render_widget(chat_box, layout[0]);

    // Input Bar
    let input_block = Block::default()
        .borders(Borders::ALL)
        .title("Message")
        .border_style(Style::default().fg(rgb_to_color(&theme.border_focus)));
    let input_para = Paragraph::new(input_value)
        .block(input_block)
        .style(Style::default().fg(rgb_to_color(&theme.text)));
    f.render_widget(input_para, layout[1]);
}

// Helper: center a rect
fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = ratatui::layout::Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);
    let vertical = popup_layout[1];
    let horizontal_layout = ratatui::layout::Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(vertical);
    horizontal_layout[1]
}

/// Launch a background websocket thread that reads/writes messages in real time.
pub fn start_ws_thread(ws_url: String, chat_tx: Sender<ChatMessage>, send_rx: Receiver<String>) {
    let token = get_token_from_file().unwrap_or_default();
    thread::spawn(move || {
        let rt = tokio::runtime::Runtime::new().expect("Failed to create runtime");
        rt.block_on(async move {
            let (ws_stream, _) = connect_async(&ws_url).await.expect("Failed to connect WS");
            let (mut ws_write, mut ws_read) = ws_stream.split();

            // Send JWT token as first message (plain text)
            ws_write
                .send(WsMessage::Text(token.into()))
                .await
                .expect("Failed to send token");

            // Spawn sender for outgoing messages
            let send_rx = send_rx;
            let mut ws_write = ws_write;
            let ws_writer = tokio::spawn(async move {
                while let Ok(msg) = send_rx.recv() {
                    if !msg.trim().is_empty() {
                        let _ = ws_write.send(WsMessage::Text(msg.into())).await;
                    }
                }
            });

            // Read incoming messages
            while let Some(msg) = ws_read.next().await {
                if let Ok(WsMessage::Text(txt)) = msg {
                    // Try parse as ChatMessage, else fallback
                    if let Ok(parsed) = serde_json::from_str::<ChatMessage>(&txt) {
                        let _ = chat_tx.send(parsed);
                    } else {
                        let _ = chat_tx.send(ChatMessage {
                            user: "system".to_string(),
                            content: txt.to_string(),
                            timestamp: Some(Local::now().format("%H:%M").to_string()),
                        });
                    }
                }
            }
            let _ = ws_writer.await;
        });
    });
}
