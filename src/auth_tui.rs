use crate::api::{login, register};
use crate::app::{App, AuthMode};
use crate::mpsc::UnboundedSender;
use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Paragraph},
    Frame,
};
use serde::Deserialize;
use std::time::Instant;
use std::{fs, path::PathBuf};

#[derive(Deserialize, Clone, Debug)]
pub struct Rgb(u8, u8, u8);

#[derive(Deserialize, Debug)]
pub struct Theme {
    pub border: Rgb,
    pub border_focus: Rgb,
    pub button: Rgb,
    pub button_focus: Rgb,
    pub text: Rgb,
}

// very unoptimized code, double function and stuff, still working on it

pub async fn handle_event(evt: Event, app: &mut App, _tx: &UnboundedSender<String>) {
    if let Event::Key(KeyEvent {
        code, modifiers, ..
    }) = evt
    {
        let reg_mode = app.auth_mode == AuthMode::Register;
        let input_count = if reg_mode { 3 } else { 2 };
        let btn_idx = input_count;

        let icon_picker_input_idx = 2;

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
                app.input_boxes[0].value.clear();
                app.input_boxes[1].value.clear();
                app.input_boxes[0].cursor = 0;
                app.input_boxes[1].cursor = 0;
            }
            KeyCode::Char('R') => {
                app.auth_mode = AuthMode::Register;
                app.focus = 0;
                app.input_boxes[0].value.clear();
                app.input_boxes[1].value.clear();
                app.input_boxes[0].cursor = 0;
                app.input_boxes[1].cursor = 0;
            }
            KeyCode::Char('Q') => {
                app.should_quit = true;
            }
            KeyCode::Char('t') if modifiers.contains(KeyModifiers::CONTROL) => {
                app.auth_mode = if app.auth_mode == AuthMode::Register {
                    AuthMode::Login
                } else {
                    AuthMode::Register
                };
                app.focus = 0;
                app.input_boxes[0].value.clear();
                app.input_boxes[1].value.clear();
                app.input_boxes[0].cursor = 0;
                app.input_boxes[1].cursor = 0;
            }
            KeyCode::Enter => {
                if app.focus == btn_idx && !app.is_loading {
                    app.is_loading = true;
                    app.error = None;
                    let username = app.input_boxes[0].value.trim().to_string();
                    let password = app.input_boxes[1].value.trim().to_string();
                    let api_base = "http://back.reetui.hackclub.app";

                    if username.is_empty() || password.is_empty() {
                        app.error = Some("Username and Password required".into());
                        app.error_time = Some(Instant::now());
                        app.input_boxes[0].value.clear();
                        app.input_boxes[1].value.clear();
                        app.input_boxes[0].cursor = 0;
                        app.input_boxes[1].cursor = 0;
                        app.is_loading = false;
                        return;
                    }

                    let res = if reg_mode {
                        register(&username, &password, &app.current_icon, api_base).await
                    } else {
                        login(&username, &password, api_base).await
                    };
                    match res {
                        Ok(token) => {
                            app.token = Some(token.token.clone());
                            app.page = crate::app::Page::Home;
                        }
                        Err(e) => {
                            let err_msg = if e.contains("409") {
                                "409: User already exists, use a pretty name :3".to_string()
                            } else if e.contains("401") {
                                "Incorrect password, ya forgot ? it's 1234 ofc".to_string()
                            } else {
                                e
                            };
                            app.error = Some(err_msg);
                            app.error_time = Some(Instant::now());
                            app.input_boxes[0].value.clear();
                            app.input_boxes[1].value.clear();
                            app.input_boxes[0].cursor = 0;
                            app.input_boxes[1].cursor = 0;
                        }
                    }
                    app.is_loading = false;
                }
            }
            KeyCode::Char(c)
                if app.focus < input_count
                    && (app.focus == 0
                        || app.focus == 1
                        || (reg_mode && app.focus == icon_picker_input_idx)) =>
            {
                app.input_boxes[app.focus].value.push(c);
                app.input_boxes[app.focus].cursor += 1;
            }
            KeyCode::Backspace
                if app.focus < input_count
                    && (app.focus == 0
                        || app.focus == 1
                        || (reg_mode && app.focus == icon_picker_input_idx)) =>
            {
                if app.input_boxes[app.focus].cursor > 0 {
                    app.input_boxes[app.focus].value.pop();
                    app.input_boxes[app.focus].cursor -= 1;
                }
            }
            KeyCode::Left if reg_mode && app.focus == icon_picker_input_idx => {
                let len = app.icons.len();
                app.icon_index = (app.icon_index + len - 1) % len;
                app.current_icon = app.icons[app.icon_index].to_string();
            }
            KeyCode::Right if reg_mode && app.focus == icon_picker_input_idx => {
                let len = app.icons.len();
                app.icon_index = (app.icon_index + 1) % len;
                app.current_icon = app.icons[app.icon_index].to_string();
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

pub fn ui(f: &mut Frame, app: &mut App) {
    let theme = get_theme();

    f.render_widget(
        Block::default().style(Style::default().bg(Color::Reset).fg(Color::Reset)),
        f.area(),
    );

    let reg_mode = app.auth_mode == AuthMode::Register;
    let visible_inputs = if reg_mode { 3 } else { 2 };
    let btn_idx = visible_inputs;
    let box_width = 38;
    let box_height = (visible_inputs as u16 * 3) + 1 + 3 + 2;

    let main_area = fixed_rect_in_center(f.area(), box_width, box_height);

    let box_title = if reg_mode { "Register" } else { "Login" };
    f.render_widget(
        Block::default()
            .title(Span::styled(
                box_title,
                Style::default()
                    .fg(rgb_to_color(&theme.text))
                    .add_modifier(Modifier::BOLD | Modifier::ITALIC),
            ))
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(rgb_to_color(&theme.border))),
        main_area,
    );

    let inner = Rect {
        x: main_area.x + 1,
        y: main_area.y + 1,
        width: main_area.width.saturating_sub(2),
        height: main_area.height.saturating_sub(2),
    };

    let mut constraints = Vec::with_capacity(visible_inputs * 3 + 2);
    for _ in 0..visible_inputs {
        constraints.push(Constraint::Length(3));
    }
    constraints.push(Constraint::Length(1));
    constraints.push(Constraint::Length(3));

    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints(constraints)
        .split(inner);

    let icon_idx = 2;

    for (idx, input) in app.input_boxes.iter().take(visible_inputs).enumerate() {
        let focus = app.focus == idx;
        let input_area = rows[idx];
        let border_style = if focus {
            Style::default()
                .fg(rgb_to_color(&theme.border_focus))
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(rgb_to_color(&theme.border))
        };

        if reg_mode && idx == icon_idx {
            let icons = &app.icons;
            let center = app.icon_index;
            let len = icons.len();

            let indices: Vec<usize> = (-2..=2)
                .map(|offset| ((center as isize + offset + len as isize) % len as isize) as usize)
                .collect();

            let mut spans = Vec::with_capacity(5 * 2 - 1);

            for (pos, &i) in indices.iter().enumerate() {
                if pos == 2 {
                    spans.push(Span::styled(
                        icons[i],
                        Style::default()
                            .fg(rgb_to_color(&theme.button_focus))
                            .add_modifier(Modifier::BOLD),
                    ));
                } else {
                    spans.push(Span::styled(
                        icons[i],
                        Style::default()
                            .fg(Color::DarkGray)
                            .add_modifier(Modifier::DIM),
                    ));
                }
                if pos != 4 {
                    spans.push(Span::raw(" "));
                }
            }

            let icon_para = Paragraph::new(Line::from(spans))
                .alignment(Alignment::Center)
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .border_type(BorderType::Rounded)
                        .border_style(border_style)
                        .title(Span::styled(
                            input.label.clone(),
                            Style::default()
                                .fg(Color::DarkGray)
                                .add_modifier(Modifier::ITALIC),
                        )),
                );
            f.render_widget(icon_para, input_area);
        } else {
            f.render_widget(
                Paragraph::new(input.display())
                    .style(Style::default().fg(rgb_to_color(&theme.text)))
                    .block(
                        Block::default()
                            .borders(Borders::ALL)
                            .border_type(BorderType::Rounded)
                            .border_style(border_style)
                            .title(Span::styled(
                                input.label.clone(),
                                Style::default()
                                    .fg(Color::DarkGray)
                                    .add_modifier(Modifier::ITALIC),
                            )),
                    )
                    .alignment(Alignment::Left),
                input_area,
            );
        }
    }

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
        Style::default()
            .fg(rgb_to_color(&theme.button))
            .add_modifier(Modifier::BOLD)
    };
    let btn_area = rows[btn_idx + 1];
    let btn_para = Paragraph::new(Span::styled(btn_label, btn_style))
        .alignment(Alignment::Center)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(if btn_focus {
                    Style::default()
                        .fg(rgb_to_color(&theme.button_focus))
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(rgb_to_color(&theme.button))
                })
                .title(""),
        );
    f.render_widget(btn_para, btn_area);

    let help_text = if reg_mode {
        "Already have an account? Press [L] to switch to Login."
    } else {
        "Don't have an account? Press [R] to switch to Register."
    };
    let help_line = Line::from(vec![
        Span::styled(
            help_text,
            Style::default()
                .fg(Color::DarkGray)
                .add_modifier(Modifier::ITALIC),
        ),
        Span::styled(
            "    Q: Quit    Tab/Shift+Tab: Move | Enter: Submit",
            Style::default().fg(Color::Gray),
        ),
    ]);
    let area = f.area();
    let bottom_area = Rect {
        x: area.x + 1,
        y: area.y + area.height.saturating_sub(2),
        width: area.width.saturating_sub(2),
        height: 1,
    };
    f.render_widget(
        Paragraph::new(help_line)
            .alignment(Alignment::Left)
            .block(Block::default().borders(Borders::NONE)),
        bottom_area,
    );

    if let Some(ref err) = app.error {
        let error_area = Rect {
            x: 1,
            y: 0,
            width: f.area().width.min(48),
            height: 1,
        };
        f.render_widget(
            Paragraph::new(Span::styled(
                err,
                Style::default()
                    .fg(Color::Red)
                    .bg(Color::Reset)
                    .add_modifier(Modifier::BOLD),
            ))
            .alignment(Alignment::Left)
            .block(Block::default().borders(Borders::NONE)),
            error_area,
        );
    }
}

fn fixed_rect_in_center(area: Rect, width: u16, height: u16) -> Rect {
    let x = area.x + (area.width.saturating_sub(width)) / 2;
    let y = area.y + (area.height.saturating_sub(height)) / 2;
    Rect::new(x, y, width.min(area.width), height.min(area.height))
}
