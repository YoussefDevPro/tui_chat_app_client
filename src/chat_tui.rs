use crate::app::App;
use crate::mpsc::UnboundedSender;
use chrono::{Local, TimeZone, Utc};
use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
use futures_util::{SinkExt, StreamExt};
use ratatui::prelude::Rect;
use ratatui::{
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, BorderType, Borders, Paragraph, Wrap},
    Frame,
};
use serde::Deserialize;
use std::time::Duration;
use std::time::Instant;
use std::{path::PathBuf, thread};
use tokio_tungstenite::{connect_async, tungstenite::Message as WsMessage};
use unicode_width::UnicodeWidthStr;

/// Chat message data structure
#[derive(Debug, Clone, serde::Deserialize)]
pub struct ChatMessage {
    pub user: String,
    pub icon: Option<String>,
    pub content: String,
    pub timestamp: Option<i64>, // Unix timestamp in seconds
}

/// RGB color structure
#[derive(Deserialize, Clone, Debug)]
pub struct Rgb(u8, u8, u8);

/// Theme color structure
#[derive(Deserialize, Debug)]
pub struct Theme {
    pub border: Rgb,
    pub border_focus: Rgb,
    pub button_focus: Rgb,
    pub text: Rgb,
}

/// Helper: Converts RGB struct to ratatui Color
fn rgb_to_color(rgb: &Rgb) -> Color {
    Color::Rgb(rgb.0, rgb.1, rgb.2)
}

/// Helper: Reads and parses theme from theme.json
fn get_theme() -> Theme {
    let theme_path = PathBuf::from("theme.json");
    let data = std::fs::read_to_string(theme_path).expect("theme.json not found");
    serde_json::from_str(&data).expect("Invalid theme.json")
}

/// Helper: Returns a relative time string for a unix timestamp
fn relative_time(ts: i64) -> String {
    let now = Utc::now().timestamp();
    let diff = now - ts;
    match diff {
        d if d < 0 => "now".to_string(),
        0 => "now".to_string(),
        1..=59 => format!("{}s ago", diff),
        60..=3599 => format!("{}m ago", diff / 60),
        3600..=86399 => format!("{}h ago", diff / 3600),
        _ => {
            let dt = Local.timestamp_opt(ts, 0).unwrap();
            dt.format("%H:%M").to_string()
        }
    }
}

/// Helper: Wraps content with a left prefix for each visual line
fn wrap_with_prefix(
    content: &str,
    width: usize,
    prefix: &str,
    style: Style,
    content_style: Style,
) -> Vec<Line<'static>> {
    let mut lines = Vec::new();
    let mut rest = content;
    while !rest.is_empty() {
        let available = width.saturating_sub(UnicodeWidthStr::width(prefix));
        let (line, next) = if rest.width() > available {
            // Find the last char that fits
            let mut cut = 0;
            for (idx, _) in rest.char_indices() {
                if rest[..idx].width() > available {
                    break;
                }
                cut = idx;
            }
            if cut == 0 {
                cut = rest.len();
            }
            (rest[..cut].to_string(), &rest[cut..])
        } else {
            (rest.to_string(), "")
        };
        let pre = Span::styled(prefix.to_string(), style);
        let ct = Span::styled(line.trim_start().to_string(), content_style);
        lines.push(Line::from(vec![pre, ct]));
        rest = next.trim_start();
    }
    lines
}

/// Helper: Splits input string into lines with a given width (up to 4 lines)
fn split_input_lines(input: &str, width: usize) -> Vec<String> {
    let mut lines = vec![];
    let mut rest = input;
    while !rest.is_empty() && lines.len() < 4 {
        let mut take = 0;
        for (i, _) in rest.char_indices() {
            if rest[..i].width() > width {
                break;
            }
            take = i;
        }
        if take == 0 && !rest.is_empty() {
            take = rest.len();
        }
        lines.push(rest[..take].to_string());
        rest = &rest[take..];
    }
    if !rest.is_empty() {
        lines.push(rest.to_string());
    }
    lines
}

/// Helper: Finds the line and column for a cursor position in the input
fn cursor_line_col(cursor: usize, lines: &[String]) -> (usize, usize) {
    let mut total = 0;
    for (i, line) in lines.iter().enumerate() {
        if cursor <= total + line.len() {
            return (i, cursor - total);
        }
        total += line.len();
    }
    (lines.len().saturating_sub(1), 0)
}

/// Spawns a websocket thread for chat communication
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

            // Send JWT token as first message (plain text)
            ws_write
                .send(WsMessage::Text(token.into()))
                .await
                .expect("Failed to send token");

            // Spawn sender for outgoing messages
            let send_fut = tokio::spawn(async move {
                while let Some(msg) = send_rx.recv().await {
                    if !msg.trim().is_empty() {
                        let _ = ws_write.send(WsMessage::Text(msg.into())).await;
                    }
                }
            });

            // Read incoming messages
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

/// Handles user input events: input editing, navigation, and chat scrolling
pub async fn handle_event(
    evt: Event,
    app: &mut App,
    tx: &UnboundedSender<String>,
    input_width: usize,
) {
    if let Event::Key(KeyEvent {
        code, modifiers, ..
    }) = evt
    {
        match code {
            // Input editing and navigation
            KeyCode::Char(c) => {
                app.chat_input.insert(app.input_cursor, c);
                app.input_cursor += 1;
            }
            KeyCode::Backspace => {
                if app.input_cursor > 0 {
                    app.chat_input.remove(app.input_cursor - 1);
                    app.input_cursor -= 1;
                }
            }
            KeyCode::Left => {
                if app.input_cursor > 0 {
                    app.input_cursor -= 1;
                }
            }
            KeyCode::Right => {
                if app.input_cursor < app.chat_input.len() {
                    app.input_cursor += 1;
                }
            }
            KeyCode::Enter => {
                let now = Instant::now();
                if let Some(last) = app.last_sent {
                    if now.duration_since(last) < Duration::from_millis(500) {
                        // Too soon, ignore send, do not clear input
                        return;
                    }
                }
                let msg = app.chat_input.trim();
                if !msg.is_empty() {
                    let _ = tx.send(msg.to_string());
                    app.chat_input.clear();
                    app.input_cursor = 0;
                    app.last_sent = Some(now);
                }
            }
            KeyCode::Esc => {
                // Optional: Clear input or implement leave chat
            }
            // Chat box scrolling (Ctrl+Up/Down)
            KeyCode::Up if modifiers.contains(KeyModifiers::CONTROL) => {
                if app.chat_scroll > 0 {
                    app.chat_scroll -= 1;
                    app.auto_scroll = false;
                }
            }

            // Scrolling down
            KeyCode::Down if modifiers.contains(KeyModifiers::CONTROL) => {
                if app.chat_scroll < app.max_scroll {
                    app.chat_scroll += 1;
                    app.auto_scroll = false;
                }
                // If we reach the bottom, enable auto-scroll
                if app.chat_scroll >= app.max_scroll {
                    app.auto_scroll = true;
                }
            }
            // Input bar cursor movement (Up/Down)
            KeyCode::Up => {
                let lines = split_input_lines(&app.chat_input, input_width);
                let (cur_line, col) = cursor_line_col(app.input_cursor, &lines);
                if cur_line > 0 {
                    let prev_line_len = lines[cur_line - 1].len();
                    app.input_cursor = lines[..cur_line - 1].iter().map(|l| l.len()).sum::<usize>()
                        + prev_line_len.min(col);
                }
            }
            KeyCode::Down => {
                let lines = split_input_lines(&app.chat_input, input_width);
                let (cur_line, col) = cursor_line_col(app.input_cursor, &lines);
                if cur_line + 1 < lines.len() {
                    let next_line_len = lines[cur_line + 1].len();
                    let before = lines[..=cur_line].iter().map(|l| l.len()).sum::<usize>();
                    app.input_cursor = before + next_line_len.min(col);
                }
            }
            _ => {}
        }
    }
}

pub fn ui(f: &mut Frame, app: &mut App, chat_messages: &[ChatMessage]) {
    let theme = get_theme();
    let area = f.area();

    // Input bar height/layout
    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(3), Constraint::Length(6)])
        .split(area);
    let input_width = layout[1].width as usize;
    let input_lines = split_input_lines(&app.chat_input, input_width);
    let input_height = input_lines.len().max(3).min(6) as u16;

    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(3), Constraint::Length(input_height)])
        .split(area);

    // Chat lines
    let mut chat_lines = Vec::new();
    let chat_area_width = layout[0].width as usize;

    for msg in chat_messages {
        let l_top = Span::styled("┌ ", Style::default().fg(Color::DarkGray));
        let icon = msg.icon.clone().unwrap_or_else(|| "󰬌".to_string());
        let icon_span = Span::styled(
            format!("{} ", icon),
            Style::default().fg(rgb_to_color(&theme.button_focus)),
        );
        let user = Span::styled(
            &msg.user,
            Style::default()
                .fg(rgb_to_color(&theme.button_focus))
                .add_modifier(Modifier::BOLD),
        );
        let ts = Span::styled(
            format!(
                "{}",
                msg.timestamp
                    .map(|t| relative_time(t))
                    .unwrap_or_else(|| "now".to_string())
            ),
            Style::default()
                .fg(Color::DarkGray)
                .add_modifier(Modifier::DIM | Modifier::ITALIC),
        );
        let header = Line::from(vec![l_top, icon_span, user, Span::raw(" "), ts]);
        chat_lines.push(header);

        let wrapped = wrap_with_prefix(
            &msg.content,
            chat_area_width,
            "│ ",
            Style::default().fg(Color::DarkGray),
            Style::default().fg(rgb_to_color(&theme.text)),
        );
        chat_lines.extend(wrapped);
    }

    // Add invisible padding at bottom
    let phantom_lines = 2;
    for _ in 0..phantom_lines {
        chat_lines.push(Line::raw(""));
    }

    let chat_area_height = layout[0].height;
    let total_lines = chat_lines.len();
    let visible_lines = chat_area_height as usize;
    let max_scroll = if visible_lines > phantom_lines {
        total_lines.saturating_sub(visible_lines - phantom_lines) as u16
    } else {
        0
    };
    if app.auto_scroll {
        app.chat_scroll = max_scroll;
    }
    app.max_scroll = max_scroll;

    let visible_chat_lines = chat_lines
        .iter()
        .skip(app.chat_scroll as usize)
        .take(visible_lines)
        .cloned()
        .collect::<Vec<_>>();

    let chat_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .title("Chat")
        .border_style(Style::default().fg(rgb_to_color(&theme.border)));
    let chat_box = Paragraph::new(visible_chat_lines)
        .block(chat_block)
        .wrap(Wrap { trim: false });
    f.render_widget(chat_box, layout[0]);

    let input_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .title("Message")
        .border_style(Style::default().fg(rgb_to_color(&theme.border_focus)));
    let input_para = Paragraph::new(app.chat_input.as_str())
        .block(input_block)
        .wrap(Wrap { trim: false })
        .style(Style::default().fg(rgb_to_color(&theme.text)))
        .scroll((0, app.input_scroll));
    f.render_widget(
        input_para,
        Rect {
            x: layout[1].x,
            y: layout[1].y,
            width: layout[1].width,
            height: input_height,
        },
    );
}
