mod api;
mod app;
mod auth_tui;
mod chat_tui;
mod home_tui;
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

// don't worry guys, im too lazy to write comments, also im trying to organize this lasagna code

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
    let (outgoing_tx, outgoing_rx) = tokio::sync::mpsc::unbounded_channel::<String>();
    let mut maybe_outgoing_rx = Some(outgoing_rx);
    let mut ws_started = false;

    let tick_rate = Duration::from_millis(100);
    let mut last_tick = Instant::now();
    let ws_url = "ws://isock.reetui.hackclub.app";

    loop {
        while let Ok(msg) = chat_rx.try_recv() {
            let mut app_lock = app.lock().unwrap();
            app_lock.chat_messages.push(msg);
            if app_lock.chat_messages.len() > 1000 {
                app_lock.chat_messages.remove(0);
            }
        }

        {
            let app_lock = app.lock().unwrap();
            if !ws_started {
                if let Some(token) = app_lock.token.clone() {
                    if let Some(rx) = maybe_outgoing_rx.take() {
                        chat_tui::start_ws_thread(ws_url.to_string(), token, chat_tx.clone(), rx);
                        ws_started = true;
                    }
                }
            }
        }

        {
            let mut app_lock = app.lock().unwrap();
            terminal.draw(|f| match app_lock.page {
                Page::Auth => auth_tui::ui(f, &mut app_lock),
                Page::Home => home_tui::ui(f, &app_lock),
                Page::Chat => {
                    let chat_messages = app_lock.chat_messages.clone();
                    chat_tui::ui(f, &mut app_lock, &chat_messages);
                }
            })?;
        }

        let timeout = tick_rate
            .checked_sub(last_tick.elapsed())
            .unwrap_or_else(|| Duration::from_secs(0));
        if event::poll(timeout)? {
            let evt = event::read()?;
            let mut app_lock = app.lock().unwrap();
            match app_lock.page {
                Page::Auth => auth_tui::handle_event(evt, &mut app_lock, &tx).await,
                Page::Home => home_tui::handle_event(evt, &mut app_lock),
                Page::Chat => {
                    let input_width = app_lock.input_width;
                    chat_tui::handle_event(evt, &mut app_lock, &outgoing_tx, input_width).await;
                }
            }
        }

        if last_tick.elapsed() >= tick_rate {
            last_tick = Instant::now();
        }
        {
            let mut app_lock = app.lock().unwrap();
            if let Some(error_time) = app_lock.error_time {
                if error_time.elapsed().as_secs() >= 3 {
                    app_lock.error = None;
                    app_lock.error_time = None;
                }
            }
        }
    }
}
