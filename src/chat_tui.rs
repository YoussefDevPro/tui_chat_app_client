use crossterm::{
    event::{self, Event, KeyCode, KeyEvent},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    text::Line,
    widgets::{Block, BorderType, Borders, List, ListItem, Paragraph},
    Frame, Terminal,
};
use std::io::{self};

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

// very basic tui for the chat, btw /quit make u quit

pub fn run_chat_tui(initial_messages: Vec<String>) -> ChatState {
    enable_raw_mode().unwrap();
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen).unwrap();
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend).unwrap();

    let mut state = ChatState::new();
    state.messages = initial_messages;

    loop {
        terminal.draw(|f| draw_chat(f, &state)).unwrap();
        if event::poll(std::time::Duration::from_millis(50)).unwrap() {
            if let Event::Key(KeyEvent { code, .. }) = event::read().unwrap() {
                match code {
                    KeyCode::Char(c) => state.input.push(c),
                    KeyCode::Backspace => {
                        state.input.pop();
                    }
                    KeyCode::Enter => {
                        if state.input == "/quit" {
                            state.done = true;
                            break;
                        }
                        if !state.input.is_empty() {
                            state.messages.push(state.input.clone());
                            state.input.clear();
                        }
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
    state
}

fn draw_chat(f: &mut Frame, state: &ChatState) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(2)
        .constraints([Constraint::Min(5), Constraint::Length(3)].as_ref())
        .split(f.area());

    let messages: Vec<ListItem> = state
        .messages
        .iter()
        .map(|msg| ListItem::new(Line::from(msg.as_str())))
        .collect();

    f.render_widget(
        Block::default()
            .title("Chat Room")
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded),
        f.area(),
    );
    f.render_widget(
        List::new(messages).block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded),
        ),
        chunks[0],
    );
    f.render_widget(
        Paragraph::new(Line::from(state.input.as_str()))
            .block(Block::default().borders(Borders::ALL).title("Input")),
        chunks[1],
    );
}
