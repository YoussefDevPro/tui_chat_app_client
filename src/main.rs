mod api;
mod app;
mod auth_tui;
mod chat_tui;
mod home_tui;
use crossterm::event::Event;
use ratatui::crossterm::{
    event::{self, EnableMouseCapture},
    execute,
    terminal::{enable_raw_mode, EnterAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::io::{self};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tokio::sync::mpsc;

use app::{App, Page};

#[tokio::main]
async fn main() -> Result<(), io::Error> {
    enable_raw_mode()?;
    let (chat_tx, chat_rx) = std::sync::mpsc::channel::<chat_tui::ChatMessage>();
    let mut stdout = std::io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let app = Arc::new(Mutex::new(App::new()));
    let (tx, _rx) = mpsc::unbounded_channel();

    let tick_rate = Duration::from_millis(100);
    let mut last_tick = Instant::now();

    loop {
        // Draw UI
        let mut app_lock = app.lock().unwrap();
        terminal.draw(|f| {
            match app_lock.page {
                Page::Auth => auth_tui::ui(f, &mut app_lock),
                Page::Home => home_tui::ui(f, &app_lock),
                Page::Chat => {
                    // You must provide chat_messages and input_value here
                    chat_tui::ui(f, &app_lock, &[], "");
                }
            }
        })?;

        // Handle input
        let timeout = tick_rate
            .checked_sub(last_tick.elapsed())
            .unwrap_or_else(|| Duration::from_secs(0));
        if event::poll(timeout)? {
            let evt = event::read()?;
            match app_lock.page {
                Page::Auth => auth_tui::handle_event(evt, &mut app_lock, &tx).await,
                Page::Home => home_tui::handle_event(evt, &mut app_lock),
                Page::Chat => chat_tui::handle_event(evt, &mut app_lock, &tx, &chat_rx).await,
            }
        }

        if last_tick.elapsed() >= tick_rate {
            last_tick = Instant::now();
        }
    }
}
