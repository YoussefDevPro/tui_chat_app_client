use crossterm::{
    event::{self, Event, KeyCode, KeyEvent},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Clear, Paragraph},
    Frame, Terminal,
};
use std::{io, process::exit};
use std::{
    time::{Duration, Instant},
    usize,
};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AuthMode {
    Register,
    Login,
}

// the auth tui, is the file i was really working on, since the first we do when we want to connect
// to discord is to create an acc or register it, so here, there is 2 mode, Register a,d Login,
// then when everything okay, we give this data to the main so it can redirect the user to the chat
// tui, gonna make the chat tui tommoroy then ill make a settings tui where we're we can change the
// icon and user name,

pub struct AuthState {
    pub username: String,
    pub password: String,
    pub icon: String,
    pub focus: u8, // 0=username, 1=password, 2=icon, 3=submit
    pub done: bool,
    pub error: Option<String>,
    pub waiting: bool,
    pub in_command_mode: bool,
    pub command_input: String,
    pub switch_to_login: bool,
    pub switch_to_register: bool,
}

impl AuthState {
    pub fn new() -> Self {
        Self {
            // init the data ofc
            username: String::new(),
            password: String::new(),
            icon: String::new(),
            focus: 0,
            done: false,
            error: None,
            waiting: false,
            in_command_mode: false,
            command_input: String::new(),
            switch_to_login: false,
            switch_to_register: false,
        }
    }
}
// for some reason nvim tell me that da 'function defined here' ?
pub fn run_auth_tui<F>(mode: AuthMode, mut submit_fn: F) -> AuthState
where
    F: FnMut(&str, &str, &str) -> Option<Result<(), String>>,
{
    enable_raw_mode().unwrap();
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen).unwrap();
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend).unwrap();

    let mut state = AuthState::new();
    let mut last_tick = Instant::now();

    loop {
        terminal.draw(|f| draw_auth(f, &state, mode)).unwrap();

        // animate cursor (not used, but preserved for future maybe)
        let tick_rate = Duration::from_millis(70);
        let now = Instant::now();
        if now.duration_since(last_tick) >= tick_rate {
            last_tick = now;
        }
        if state.waiting {
            if let Some(reg_result) = submit_fn(&state.username, &state.password, &state.icon) {
                match reg_result {
                    Ok(()) => {
                        state.done = true;
                        break;
                    }
                    Err(e) => {
                        state.error = Some(e);
                        state.waiting = false;
                    }
                }
            }
        }
        let last_focus = if mode == AuthMode::Register { 3 } else { 2 };
        if event::poll(Duration::from_millis(30)).unwrap() {
            if let Event::Key(KeyEvent { code, .. }) = event::read().unwrap() {
                if state.in_command_mode {
                    match code {
                        KeyCode::Esc | KeyCode::Char('q') => {
                            state.in_command_mode = false;
                            state.command_input.clear();
                            exit(0);
                        }
                        KeyCode::Backspace => {
                            state.command_input.pop();
                        }
                        KeyCode::Enter => {
                            state.in_command_mode = false;
                            state.command_input.clear();
                        }
                        KeyCode::Char('L') => {
                            // btw, 'L' mean Login
                            state.in_command_mode = false;
                            state.command_input.clear();
                            state.switch_to_login = true;
                            state.switch_to_register = false;
                            break;
                        }
                        KeyCode::Char('R') => {
                            state.in_command_mode = false;
                            state.command_input.clear();
                            state.switch_to_register = true;
                            state.switch_to_login = false;
                            break;
                        }
                        KeyCode::Char(c) => {
                            state.command_input.push(c);
                        }
                        _ => {}
                    }
                    continue; // if we are in cmd mode we don't execute other commands, like nvim
                }
                if let KeyCode::Char(':') = code {
                    // u don't use nvim ?
                    state.in_command_mode = true;
                    state.command_input.clear();
                    continue;
                }
                if state.error.is_some() {
                    state.error = None;
                    continue;
                }
                match state.focus {
                    0 => match code {
                        KeyCode::Tab | KeyCode::Down => state.focus = 1,
                        KeyCode::Enter => state.focus = 1,
                        KeyCode::Char(c) => state.username.push(c),
                        KeyCode::Backspace => {
                            state.username.pop();
                        }
                        _ => {}
                    },
                    1 => match code {
                        KeyCode::Tab | KeyCode::Down => state.focus = 2,
                        KeyCode::Up => state.focus = 0,
                        KeyCode::Enter => state.focus = 2,
                        KeyCode::Char(c) => state.password.push(c),
                        KeyCode::Backspace => {
                            state.password.pop();
                        }
                        _ => {}
                    },
                    2 => {
                        if mode == AuthMode::Register {
                            match code {
                                KeyCode::Tab | KeyCode::Down | KeyCode::Enter => {
                                    state.focus = last_focus
                                }
                                KeyCode::Up => state.focus = 1,
                                KeyCode::Char(c) => state.icon.push(c),
                                KeyCode::Backspace => {
                                    state.icon.pop();
                                }
                                _ => {}
                            } // if we are in login mode, the icon input bar don't show, then the
                              // index of the button will be 2 and not 3
                        } else {
                            // In Login mode, focus 2 is actually the submit button
                            match code {
                                KeyCode::Up => state.focus = 1,
                                KeyCode::Enter => state.waiting = true,
                                _ => {}
                            }
                        }
                    }
                    3 => {
                        // only in register mode
                        match code {
                            KeyCode::Up => state.focus = 2,
                            KeyCode::Enter => state.waiting = true,
                            _ => {}
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

// this function draw everything, and it is quite not understandable
// and this is who cause the file to be that long
// bc ad u know, humanity over complcate him self
// like, imagine, my first css file was 978 lines long, crazy right !
fn draw_auth(f: &mut Frame, state: &AuthState, mode: AuthMode) {
    let area = centered_rect(40, 50, f.area());
    let button_index = if mode == AuthMode::Register { 3 } else { 2 };
    // the whole auth box
    f.render_widget(
        Block::default()
            .title(Span::styled(
                " register to chat ",
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ))
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .style(Style::default().fg(Color::White)),
        area,
    );
    let mut constraints = vec![
        Constraint::Length(3), // susername
        Constraint::Length(3), // password
    ];
    if mode == AuthMode::Register {
        constraints.push(Constraint::Length(3)); // econ for register mode
    }
    constraints.push(Constraint::Length(3)); // susbmit button

    let inner = Layout::default()
        .direction(Direction::Vertical)
        .margin(2)
        .constraints(constraints)
        .split(area);

    // im being lazy after eating pizza
    let get_border_style = |field: u8| {
        if state.focus == field {
            Style::default()
                .fg(Color::LightGreen)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::DarkGray)
        }
    };

    // susername
    f.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled(
                "",
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                &state.username,
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
            ),
        ]))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(get_border_style(0))
                .title(Span::styled(
                    "   username ",
                    Style::default()
                        .fg(Color::Green)
                        .add_modifier(Modifier::BOLD),
                )),
        )
        .alignment(Alignment::Left),
        shrink_horiz(inner[0], 36),
    );
    // pasusword
    f.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled(
                "",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                &"*".repeat(state.password.len()),
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
            ),
        ]))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(get_border_style(1))
                .title(Span::styled(
                    "  password ",
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD),
                )),
        )
        .alignment(Alignment::Left),
        shrink_horiz(inner[1], 36),
    );
    // econ
    if mode == AuthMode::Register {
        f.render_widget(
            Paragraph::new(Line::from(vec![
                Span::styled(
                    "",
                    Style::default()
                        .fg(Color::Magenta)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    &state.icon,
                    Style::default()
                        .fg(Color::White)
                        .add_modifier(Modifier::BOLD),
                ),
            ]))
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .border_style(get_border_style(2))
                    .title(Span::styled(
                        "   icon ",
                        Style::default()
                            .fg(Color::Magenta)
                            .add_modifier(Modifier::BOLD),
                    )),
            )
            .alignment(Alignment::Left),
            shrink_horiz(inner[2], 36),
        );
    }
    // SuSbmit
    let submit_label = if state.waiting {
        if mode == AuthMode::Register {
            "registering..."
        } else {
            "logging in..."
        }
    } else {
        if mode == AuthMode::Register {
            "register (enter)"
        } else {
            "login (enter)"
        }
    };
    f.render_widget(
        Paragraph::new(Line::from(vec![Span::styled(
            submit_label,
            Style::default()
                .fg(Color::Blue)
                .add_modifier(Modifier::BOLD),
        )]))
        .alignment(Alignment::Center)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(get_border_style(button_index)),
        ),
        shrink_horiz(inner[button_index as usize], 24),
    );

    // E(rror) popup
    if let Some(err) = &state.error {
        let popup = centered_rect(40, 20, f.area());
        f.render_widget(Clear, popup);
        f.render_widget(
            Block::default()
                .title(Span::styled(
                    "error",
                    Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
                ))
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .style(Style::default().fg(Color::Red).bg(Color::Rgb(0, 0, 0))),
            popup,
        );
        let text = Paragraph::new(Line::from(vec![Span::styled(
            err,
            Style::default()
                .fg(Color::White)
                .bg(Color::Black)
                .add_modifier(Modifier::BOLD),
        )]))
        .alignment(Alignment::Center)
        .block(Block::default());
        f.render_widget(
            text,
            popup.inner(ratatui::layout::Margin {
                vertical: 2,
                horizontal: 2,
            }),
        );
    }
    // the cmd line
    if state.in_command_mode {
        let popup_width = 20;
        let popup_height = 3;
        let area = f.area();
        let popup = Rect {
            x: area.x + (area.width.saturating_sub(popup_width)) / 2,
            y: area.y,
            width: popup_width,
            height: popup_height,
        };

        f.render_widget(
            ratatui::widgets::Block::default().style(Style::default().bg(Color::DarkGray)),
            area,
        );

        f.render_widget(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .title(Span::styled(
                    "Command",
                    Style::default()
                        .fg(Color::White)
                        .add_modifier(Modifier::BOLD),
                ))
                .style(Style::default().bg(Color::Black)),
            popup,
        );
        f.render_widget(
            Paragraph::new(Line::from(vec![
                Span::styled(
                    "  ",
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::raw(&state.command_input),
            ]))
            .style(Style::default().fg(Color::White))
            .alignment(Alignment::Left),
            Rect {
                x: popup.x + 2,
                y: popup.y + popup.height - 2,
                width: popup.width - 4,
                height: 1,
            },
        );
    }
    let switch_hint = match mode {
        AuthMode::Register => "Already have an account? Press :L to login.",
        AuthMode::Login => "Don't have an account? Press :R to register.",
    };

    f.render_widget(
        Paragraph::new(Line::from(vec![Span::styled(
            switch_hint,
            Style::default().fg(Color::Yellow),
        )]))
        .alignment(Alignment::Center),
        Rect {
            x: area.x,
            y: area.y + 17, // hardcoded value that can not work in other screen than a X230
            // thinkpad
            width: area.width,
            height: 1,
        },
    );
}

// who need to center a div if we can center a tui box
fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints(
            [
                Constraint::Percentage((100 - percent_y) / 2),
                Constraint::Percentage(percent_y),
                Constraint::Percentage((100 - percent_y) / 2),
            ]
            .as_ref(),
        )
        .split(r);
    let vertical = popup_layout[1];
    let horizontal_layout = Layout::default()
        .direction(Direction::Horizontal)
        .constraints(
            [
                Constraint::Percentage((100 - percent_x) / 2),
                Constraint::Percentage(percent_x),
                Constraint::Percentage((100 - percent_x) / 2),
            ]
            .as_ref(),
        )
        .split(vertical);
    horizontal_layout[1]
}
// centering
fn shrink_horiz(r: Rect, width: u16) -> Rect {
    let w = r.width.min(width);
    let x = r.x + (r.width - w) / 2;
    Rect {
        x,
        y: r.y,
        width: w,
        height: r.height,
    }
}
