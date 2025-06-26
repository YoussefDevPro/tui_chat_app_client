use crate::api::login_or_register;
use crate::app::{App, AuthMode};
use crate::mpsc::UnboundedSender;
use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
use directories::ProjectDirs;
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Clear, Paragraph},
    Frame,
};
use serde::Deserialize;
use std::{fs, path::PathBuf};

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
    pub input_hover: Rgb,
}

pub async fn handle_event(evt: Event, app: &mut App, _tx: &UnboundedSender<String>) {
    if app.show_cmd_popup {
        if let Event::Key(KeyEvent { code, .. }) = evt {
            match code {
                KeyCode::Esc => {
                    app.show_cmd_popup = false;
                    app.cmd_input.clear();
                }
                KeyCode::Enter => {
                    let cmd = app.cmd_input.trim();
                    if cmd == "q" || cmd == "quit" {
                        app.should_quit = true;
                    }
                    app.show_cmd_popup = false;
                    app.cmd_input.clear();
                }
                KeyCode::Char(c) => {
                    app.cmd_input.push(c);
                }
                KeyCode::Backspace => {
                    app.cmd_input.pop();
                }
                _ => {}
            }
        }
        return;
    }

    if let Event::Key(KeyEvent {
        code, modifiers, ..
    }) = evt
    {
        let reg_mode = app.auth_mode == AuthMode::Register;
        let input_count = if reg_mode { 3 } else { 2 };
        let btn_idx = input_count;

        match code {
            KeyCode::Tab => {
                app.focus = (app.focus + 1) % (input_count + 1);
            }
            KeyCode::BackTab | KeyCode::Up => {
                if app.focus == 0 {
                    app.focus = input_count;
                } else {
                    app.focus -= 1;
                }
            }
            KeyCode::Down => {
                app.focus = (app.focus + 1) % (input_count + 1);
            }
            KeyCode::Char('L') => {
                app.auth_mode = AuthMode::Login;
                app.focus = 0;
            }
            KeyCode::Char('R') => {
                app.auth_mode = AuthMode::Register;
                app.focus = 0;
            }
            KeyCode::Char(':') => {
                app.show_cmd_popup = true;
                app.cmd_input.clear();
            }
            KeyCode::Char('T') if modifiers.contains(KeyModifiers::CONTROL) => {
                app.auth_mode = if app.auth_mode == AuthMode::Register {
                    AuthMode::Login
                } else {
                    AuthMode::Register
                };
                app.focus = 0;
            }
            KeyCode::Enter => {
                if app.focus == btn_idx && !app.is_loading {
                    app.is_loading = true;
                    app.error = None;
                    let username = app.input_boxes[0].value.trim().to_string();
                    let password = app.input_boxes[1].value.trim().to_string();
                    let api_base = "http://localhost:8000";

                    if username.is_empty() || password.is_empty() {
                        app.error = Some("Username and Password required".into());
                        app.is_loading = false;
                        return;
                    }

                    let res = login_or_register(&username, &password, api_base).await;
                    match res {
                        Ok(token) => {
                            app.token = Some(token.clone());
                            app.page = crate::app::Page::Home;
                        }
                        Err(e) => {
                            app.error = Some(e);
                        }
                    }
                    app.is_loading = false;
                }
            }
            KeyCode::Char(c) if app.focus < input_count => {
                app.input_boxes[app.focus].value.push(c);
                app.input_boxes[app.focus].cursor += 1;
            }
            KeyCode::Backspace if app.focus < input_count => {
                if app.input_boxes[app.focus].cursor > 0 {
                    app.input_boxes[app.focus].value.pop();
                    app.input_boxes[app.focus].cursor -= 1;
                }
            }
            _ => {}
        }
    }
}

fn rgb_to_color(rgb: &Rgb) -> Color {
    Color::Rgb(rgb.0, rgb.1, rgb.2)
}

fn get_theme() -> Theme {
    let theme_path = PathBuf::from("theme.json");
    let data = fs::read_to_string(theme_path).expect("theme.json not found");
    serde_json::from_str(&data).expect("Invalid theme.json")
}

/// Returns the config dir for .config/reetui or Windows equivalent
pub fn config_dir() -> PathBuf {
    if let Some(proj_dirs) = ProjectDirs::from("com", "ReeTui", "ReeTui") {
        proj_dirs.config_dir().to_path_buf()
    } else {
        PathBuf::from(".")
    }
}

pub fn save_token(token: &str) {
    let dir = config_dir();
    if !dir.exists() {
        let _ = fs::create_dir_all(&dir);
    }
    let mut path = dir.clone();
    path.push("token");
    fs::write(path, token).unwrap();
}

pub fn ui(f: &mut Frame, app: &mut App) {
    let theme = get_theme();

    // Blank background
    let blank = Block::default()
        .style(Style::default().bg(Color::Reset).fg(Color::Reset))
        .borders(Borders::NONE);
    f.render_widget(blank, f.area());

    // Main centered box (percent for centering)
    let main_area = centered_rect(50, 50, f.area());

    // Layout: [inputs][button][help]
    let reg_mode = app.auth_mode == AuthMode::Register;
    let input_count = if reg_mode { 3 } else { 2 };
    let mut constraints = vec![Constraint::Length(4); input_count];
    constraints.push(Constraint::Length(4)); // Button
    constraints.push(Constraint::Length(2)); // Help row
    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints(constraints)
        .margin(2)
        .split(main_area);

    // Box with rounded border
    let block = Block::default()
        .title(Span::styled(
            if reg_mode { "Register" } else { "Login" },
            Style::default()
                .fg(rgb_to_color(&theme.text))
                .add_modifier(Modifier::BOLD),
        ))
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(rgb_to_color(&theme.border)));
    f.render_widget(Clear, main_area);
    f.render_widget(block, main_area);

    // Input fields (fixed width/height)
    for idx in 0..input_count {
        let focus = app.focus == idx;
        let style = if focus {
            Style::default()
                .fg(rgb_to_color(&theme.text))
                .bg(rgb_to_color(&theme.input_hover))
        } else {
            Style::default().fg(rgb_to_color(&theme.text))
        };
        let border_style = if focus {
            Style::default()
                .fg(rgb_to_color(&theme.border_focus))
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(rgb_to_color(&theme.border))
        };
        let input = &app.input_boxes[idx];
        let para = Paragraph::new(format!("{}: {}", input.label, input.display()))
            .style(style)
            .alignment(Alignment::Left)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .border_style(border_style)
                    .title(Span::styled("", Style::default())),
            );
        let field_area = fixed_rect_in_center(layout[idx], 14, 3);
        f.render_widget(para, field_area);
    }

    // Button (fixed)
    let btn_idx = input_count;
    let btn_focus = app.focus == btn_idx;
    let btn_label = if reg_mode {
        if app.is_loading {
            "Registering..."
        } else {
            "Register"
        }
    } else {
        if app.is_loading {
            "Logging in..."
        } else {
            "Login"
        }
    };
    let btn_style = if btn_focus {
        Style::default()
            .fg(rgb_to_color(&theme.button_focus))
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(rgb_to_color(&theme.button))
    };
    let btn = Paragraph::new(Span::styled(btn_label, btn_style))
        .alignment(Alignment::Center)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(btn_style),
        );
    let btn_area = fixed_rect_in_center(layout[btn_idx], 20, 4);
    f.render_widget(btn, btn_area);

    // Help label at the bottom of the box (left aligned)
    let help_text = if reg_mode {
        "Already have an account? Press [L] to switch to Login."
    } else {
        "Don't have an account? Press [R] to switch to Register."
    };
    let help_line = Line::from(vec![
        Span::styled(help_text, Style::default().fg(Color::DarkGray)),
        Span::styled(
            "   : for command   Tab/Shift+Tab: Move | Enter: Submit",
            Style::default().fg(Color::Gray),
        ),
    ]);
    let para = Paragraph::new(help_line)
        .alignment(Alignment::Left)
        .block(Block::default().borders(Borders::NONE));
    f.render_widget(para, layout[input_count + 1]);

    // Error popup
    if let Some(ref err) = app.error {
        let popup_area = centered_rect(40, 5, f.area());
        let err_block = Block::default()
            .title("Error")
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .style(
                Style::default()
                    .fg(rgb_to_color(&theme.error_fg))
                    .bg(rgb_to_color(&theme.error_bg)),
            );
        let err_para = Paragraph::new(err.as_str())
            .alignment(Alignment::Center)
            .block(err_block);
        f.render_widget(Clear, popup_area);
        f.render_widget(err_para, popup_area);
    }

    // Command popup
    if app.show_cmd_popup {
        let popup_area = centered_rect(30, 5, f.area());
        let block = Block::default()
            .title("Command")
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(Color::White));
        let cmd_line = Line::from(vec![
            Span::styled(":", Style::default().fg(Color::Yellow)),
            Span::raw(&app.cmd_input),
        ]);
        let para = Paragraph::new(cmd_line)
            .alignment(Alignment::Left)
            .block(block);
        f.render_widget(Clear, popup_area);
        f.render_widget(para, popup_area);
    }
}

/// Helper: center a rect (percent)
fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);
    let vertical = popup_layout[1];
    let horizontal_layout = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(vertical);
    horizontal_layout[1]
}

/// Helper: give a rect of fixed size (w, h) centered inside parent
fn fixed_rect_in_center(area: Rect, width: u16, height: u16) -> Rect {
    let x = area.x + (area.width.saturating_sub(width)) / 2;
    let y = area.y + (area.height.saturating_sub(height)) / 2;
    Rect::new(x, y, width.min(area.width), height.min(area.height))
}
