use ratatui::prelude::Rect;
use ratatui::{
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Paragraph, Wrap},
    Frame,
};
use unicode_width::UnicodeWidthStr;

use crate::app::App;
use crate::chat_tui::utils::wrap_with_prefixes;

pub use self::data::ChatMessage;
pub use self::events::handle_event;
pub use self::utils::{cursor_line_col, get_theme, relative_time, rgb_to_color, split_input_lines};
pub use self::websocket::start_ws_thread;

mod data;
mod events;
mod utils;
mod websocket;
// also known as wesock

// this function draw the whole freaking thing
pub fn ui(f: &mut Frame, app: &mut App, chat_messages: &[ChatMessage]) {
    let theme = get_theme();
    let area = f.area();

    let input_width = area.width as usize;
    let input_lines_for_height_calc =
        split_input_lines(&app.chat_input, input_width.saturating_sub(2));
    let input_height = input_lines_for_height_calc.len().max(1).min(6) as u16 + 2;

    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(3), Constraint::Length(input_height)])
        .split(area);

    let mut chat_lines = Vec::new();
    let chat_area_width_for_content = layout[0].width.saturating_sub(2) as usize;

    let mut last_user: Option<String> = None;

    for msg in chat_messages {
        let is_same_user = last_user.as_ref().map_or(false, |u| u == &msg.user);

        let timestamp_str = msg
            .timestamp
            .map(|t| relative_time(t))
            .unwrap_or_else(|| " ".to_string());

        let ts_span = Span::styled(
            timestamp_str,
            Style::default()
                .fg(Color::DarkGray)
                .add_modifier(Modifier::DIM | Modifier::ITALIC),
        );

        if !is_same_user {
            let l_top = Span::styled("┌ ", Style::default().fg(Color::DarkGray));
            let icon = msg.icon.clone().unwrap_or_else(|| "󰬌".to_string());
            let icon_span_str = format!("{} ", icon);
            let user_str = &msg.user;

            let fixed_prefix_width = UnicodeWidthStr::width("┌ ");
            let icon_user_width = UnicodeWidthStr::width(icon_span_str.as_str())
                + UnicodeWidthStr::width(user_str.as_str());

            let mut header_spans = vec![
                l_top,
                Span::styled(
                    icon_span_str,
                    Style::default().fg(rgb_to_color(&theme.button_focus)),
                ),
                Span::styled(
                    user_str,
                    Style::default()
                        .fg(rgb_to_color(&theme.button_focus))
                        .add_modifier(Modifier::BOLD),
                ),
            ];

            let header_content_current_width = fixed_prefix_width + icon_user_width;
            let needed_padding_width = chat_area_width_for_content
                .saturating_sub(header_content_current_width)
                .saturating_sub(ts_span.width())
                .saturating_sub(1);

            if needed_padding_width > 0 {
                header_spans.push(Span::raw(" ".repeat(needed_padding_width)));
            } else {
                if chat_area_width_for_content > header_content_current_width + ts_span.width() {
                    header_spans.push(Span::raw(" "));
                }
            }
            header_spans.push(ts_span);

            chat_lines.push(Line::from(header_spans));

            let wrapped_content = wrap_with_prefixes(
                &msg.content,
                chat_area_width_for_content,
                "│ ",
                Style::default().fg(Color::DarkGray),
                Style::default().fg(rgb_to_color(&theme.text)),
            );
            chat_lines.extend(wrapped_content);
        } else {
            let wrapped_content = wrap_with_prefixes(
                &msg.content,
                chat_area_width_for_content,
                "│ ",
                Style::default().fg(Color::DarkGray),
                Style::default().fg(rgb_to_color(&theme.text)),
            );
            chat_lines.extend(wrapped_content);
        }
        last_user = Some(msg.user.clone());
    }

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

    let (cursor_line, cursor_col) = cursor_line_col(app.input_cursor, &input_lines_for_height_calc);
    let input_para = Paragraph::new(app.chat_input.as_str())
        .block(input_block)
        .wrap(Wrap { trim: false })
        .style(Style::default().fg(rgb_to_color(&theme.text)))
        .scroll((cursor_line as u16, 0));

    f.render_widget(
        input_para,
        Rect {
            x: layout[1].x,
            y: layout[1].y,
            width: layout[1].width,
            height: input_height,
        },
    );

    if app.cursor_tick_state {
        let cursor_x = layout[1].x + (cursor_col as u16) + 1;
        let cursor_y = layout[1].y + (cursor_line as u16) + 1;

        f.set_cursor_position((cursor_x, cursor_y));
    }
}
