use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
use std::time::{Duration, Instant};
use tokio::sync::mpsc::UnboundedSender;
use unicode_width::UnicodeWidthChar;

use super::utils::{cursor_line_col, split_input_lines};
use crate::app::App;

// Key Actions:
// Character input -> Inserts character at cursor position
// Backspace -> Deletes the character before the cursor
// Delete -> Deletes the character at the cursor position
// Left Arrow -> Moves the cursor one character to the left
// Right Arrow -> Moves the cursor one character to the right
// Home -> Moves the cursor to the beginning of the current line
// End -> Moves the cursor to the end of the current line
// Enter -> Sends the message (unless Shift is held)
// Shift + Enter -> Inserts a newline
// (Implicit) Fast Enter -> Prevents spamming messages (if pressed too quickly)
// Escape -> Currently does nothing
// Ctrl + Up Arrow -> Scrolls chat content up
// Ctrl + Down Arrow -> Scrolls chat content down
// (Implicit) Scrolling to bottom -> Re-enables auto-scroll
// Up Arrow (in input) -> Moves the input cursor up one line
// Down Arrow (in input) -> Moves the input cursor down one line
// Other keys -> Catches any other unhandled key presses

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
            KeyCode::Char(c) => {
                app.chat_input.insert(app.input_cursor, c);
                app.input_cursor += c.len_utf8();
            }
            KeyCode::Backspace => {
                if app.input_cursor > 0 {
                    let mut char_to_remove_start_byte_idx = 0;
                    let mut prev_char_len_bytes = 0;
                    for (idx, ch) in app.chat_input.char_indices() {
                        if idx + ch.len_utf8() == app.input_cursor {
                            char_to_remove_start_byte_idx = idx;
                            prev_char_len_bytes = ch.len_utf8();
                            break;
                        }
                    }
                    if prev_char_len_bytes > 0 {
                        app.chat_input.remove(char_to_remove_start_byte_idx);
                        app.input_cursor -= prev_char_len_bytes;
                    }
                }
            }
            KeyCode::Delete => {
                if app.input_cursor < app.chat_input.len() {
                    let mut char_to_remove_start_byte_idx = 0;
                    let mut char_len_bytes = 0;
                    for (idx, ch) in app.chat_input.char_indices() {
                        if idx == app.input_cursor {
                            char_to_remove_start_byte_idx = idx;
                            char_len_bytes = ch.len_utf8();
                            break;
                        }
                    }
                    if char_len_bytes > 0 {
                        app.chat_input.remove(char_to_remove_start_byte_idx);
                    }
                }
            }
            KeyCode::Left => {
                if app.input_cursor > 0 {
                    app.input_cursor = app.chat_input[..app.input_cursor]
                        .char_indices()
                        .last()
                        .map_or(0, |(idx, _)| idx);
                }
            }
            KeyCode::Right => {
                if app.input_cursor < app.chat_input.len() {
                    app.input_cursor = app.chat_input[app.input_cursor..]
                        .char_indices()
                        .next()
                        .map_or(app.chat_input.len(), |(idx, ch)| {
                            app.input_cursor + idx + ch.len_utf8()
                        });
                }
            }
            KeyCode::Home => {
                let lines = split_input_lines(&app.chat_input, input_width);
                let (cur_line_idx, _) = cursor_line_col(app.input_cursor, &lines);
                let mut new_cursor = 0;
                for i in 0..cur_line_idx {
                    new_cursor += lines[i].len();
                }
                app.input_cursor = new_cursor;
            }
            KeyCode::End => {
                let lines = split_input_lines(&app.chat_input, input_width);
                let (cur_line_idx, _) = cursor_line_col(app.input_cursor, &lines);
                let mut new_cursor = 0;
                for i in 0..=cur_line_idx {
                    new_cursor += lines[i].len();
                }
                if cur_line_idx < lines.len() - 1 {
                    if app.chat_input.bytes().nth(new_cursor.saturating_sub(1)) == Some(b'\n') {
                        app.input_cursor = new_cursor.saturating_sub(1);
                    } else {
                        app.input_cursor = new_cursor;
                    }
                } else {
                    app.input_cursor = new_cursor;
                }
            }
            KeyCode::Enter => {
                if modifiers.contains(KeyModifiers::SHIFT) {
                    app.chat_input.insert(app.input_cursor, '\n');
                    app.input_cursor += '\n'.len_utf8();
                } else {
                    let now = Instant::now();
                    if let Some(last) = app.last_sent {
                        if now.duration_since(last) < Duration::from_millis(500) {
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
            }
            KeyCode::Esc => {}
            KeyCode::Up if modifiers.contains(KeyModifiers::CONTROL) => {
                if app.chat_scroll > 0 {
                    app.chat_scroll -= 1;
                    app.auto_scroll = false;
                }
            }
            KeyCode::Down if modifiers.contains(KeyModifiers::CONTROL) => {
                if app.chat_scroll < app.max_scroll {
                    app.chat_scroll += 1;
                    app.auto_scroll = false;
                }
                if app.chat_scroll >= app.max_scroll {
                    app.auto_scroll = true;
                }
            }
            KeyCode::Up => {
                let lines = split_input_lines(&app.chat_input, input_width);
                let (cur_line, col) = cursor_line_col(app.input_cursor, &lines);
                if cur_line > 0 {
                    let mut new_cursor_byte_idx = 0;
                    for i in 0..(cur_line - 1) {
                        new_cursor_byte_idx += lines[i].len()
                            + if i < lines.len() - 1
                                && app
                                    .chat_input
                                    .bytes()
                                    .nth(new_cursor_byte_idx + lines[i].len())
                                    == Some(b'\n')
                            {
                                1
                            } else {
                                0
                            };
                    }
                    let target_line = &lines[cur_line - 1];
                    let mut current_visual_width = 0;
                    let mut char_idx_in_target_line = 0;
                    for (byte_idx, char_val) in target_line.char_indices() {
                        let char_display_width = UnicodeWidthChar::width(char_val).unwrap_or(0);
                        if current_visual_width + char_display_width > col {
                            break;
                        }
                        current_visual_width += char_display_width;
                        char_idx_in_target_line = byte_idx + char_val.len_utf8();
                    }
                    app.input_cursor = new_cursor_byte_idx + char_idx_in_target_line;
                } else {
                    app.input_cursor = 0;
                }
            }
            KeyCode::Down => {
                let lines = split_input_lines(&app.chat_input, input_width);
                let (cur_line, col) = cursor_line_col(app.input_cursor, &lines);
                if cur_line + 1 < lines.len() {
                    let mut new_cursor_byte_idx = 0;
                    for i in 0..(cur_line + 1) {
                        new_cursor_byte_idx += lines[i].len()
                            + if i < lines.len() - 1
                                && app
                                    .chat_input
                                    .bytes()
                                    .nth(new_cursor_byte_idx + lines[i].len())
                                    == Some(b'\n')
                            {
                                1
                            } else {
                                0
                            };
                    }
                    let target_line = &lines[cur_line + 1];
                    let mut current_visual_width = 0;
                    let mut char_idx_in_target_line = 0;
                    for (byte_idx, char_val) in target_line.char_indices() {
                        let char_display_width = UnicodeWidthChar::width(char_val).unwrap_or(0);
                        if current_visual_width + char_display_width > col {
                            break;
                        }
                        current_visual_width += char_display_width;
                        char_idx_in_target_line = byte_idx + char_val.len_utf8();
                    }
                    app.input_cursor = new_cursor_byte_idx + char_idx_in_target_line;
                } else {
                    app.input_cursor = app.chat_input.len();
                }
            }
            _ => {}
        }
    }
}
