use crate::client::{send_message, NetDebug};
use crossterm::{
    event::{self, Event, KeyCode, KeyEvent},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, List, ListItem, Paragraph},
    Frame, Terminal,
};
use std::io;
use std::sync::Arc;
use std::sync::Mutex;
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, BufReader, Lines};
use tokio::net::tcp::{OwnedReadHalf, OwnedWriteHalf};

pub struct ChatState {
    pub input: String,
    pub messages: Vec<String>,
    pub done: bool,
}

impl ChatState {
    pub fn new() -> Self {
        Self {
            input: String::new(),
            messages: Vec::new(),
            done: false,
        }
    }
}

/// Run the chat TUI. This is an async function.
/// Arguments:
/// - `write_half`: OwnedWriteHalf for sending messages to server
/// - `socket_lines`: mutable reference to Lines<BufReader<OwnedReadHalf>> for reading server responses
/// - `user_id`: String id of the user
/// - `net_debug`: NetDebug instance for logging
pub async fn run_chat_tui(
    mut write_half: OwnedWriteHalf,
    messages: Arc<Mutex<Vec<String>>>,
    user_id: String,
    net_debug: NetDebug,
) {
    enable_raw_mode().unwrap();
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen).unwrap();
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend).unwrap();

    let mut state = ChatState::new();
    state
        .messages
        .push("Welcome! Type and press Enter to send. /quit to exit.".into());

    loop {
        terminal.draw(|f| draw_chat(f, &state)).unwrap();
        if event::poll(Duration::from_millis(50)).unwrap() {
            if let Event::Key(KeyEvent { code, .. }) = event::read().unwrap() {
                match code {
                    KeyCode::Char(c) => state.input.push(c),
                    KeyCode::Backspace => {
                        state.input.pop();
                    }
                    KeyCode::Enter => {
                        let input = state.input.trim().to_string();
                        if input == "/quit" {
                            state.done = true;
                            break;
                        }
                        if !input.is_empty() {
                            // Send to server and show raw JSON response in chat
                            let resp =
                                send_message(&user_id, &input, &mut write_half, &net_debug).await;
                            println!("Registration response: {:?}", resp);
                            match resp {
                                Ok(_json) => {
                                    state.messages.push(format!("(sent) {}", input));
                                    state.messages.push(format!("(server) sent: {}", input));
                                }
                                Err(e) => {
                                    state.messages.push(format!("(error) {}", e));
                                }
                            }
                            state.input.clear();
                        }
                    }
                    KeyCode::Esc => {
                        state.done = true;
                        break;
                    }
                    _ => {}
                }
            }
        }
        if state.done {
            break;
        }
    }
    disable_raw_mode().unwrap();
    execute!(io::stdout(), LeaveAlternateScreen).unwrap();
}

fn draw_chat(f: &mut Frame, state: &ChatState) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(2)
        .constraints([Constraint::Min(5), Constraint::Length(3)].as_ref())
        .split(f.area());

    // Show last 20 messages
    let messages: Vec<ListItem> = state
        .messages
        .iter()
        .rev()
        .take(20)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .map(|msg| ListItem::new(Line::from(msg.as_str())))
        .collect();

    f.render_widget(
        Block::default()
            .title("Chat")
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded),
        chunks[0],
    );
    f.render_widget(
        List::new(messages).block(Block::default().borders(Borders::NONE)),
        chunks[0],
    );
    f.render_widget(
        Paragraph::new(Line::from(state.input.as_str()))
            .block(Block::default().borders(Borders::ALL).title("Input")),
        chunks[1],
    );
}
// brain storming
// okay so here im going to do some tanstition between the auth page and the chat page, and even
// the settings page, why not! also i think ill add a bot api, yeah, or no, idk ill see tommoroy if
// ill do it or not, i got an idea, why not doing themes, loaded from a file, yeah, like, ratatui
// gives tone of themes for the border etc, i will be nice, what next, hmmmmmm, i think ill also
// work a bit in the security side, ive realised hat we send the data to the server, like, if we
// send a password a man in the middle can intercept he password, so we have to send it already
// hashed, then stock it as plain text (just kidding, or not)
