use chrono::{Local, TimeZone, Utc};
use ratatui::{
    style::{Color, Style},
    text::{Line, Span},
};
use std::path::PathBuf;
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

use super::data::{Rgb, Theme};

// this file have some utils, ill soon make the auth tui and the home tui in this, and rename it
// app, or maiw

pub fn rgb_to_color(rgb: &Rgb) -> Color {
    Color::Rgb(rgb.0, rgb.1, rgb.2)
}

pub fn get_theme() -> Theme {
    let theme_path = PathBuf::from("theme.json");
    let data = std::fs::read_to_string(theme_path).expect("theme.json not found");
    serde_json::from_str(&data).expect("Invalid theme.json")
}

pub fn relative_time(ts: i64) -> String {
    let now = Utc::now().timestamp();
    let diff = now - ts;
    match diff {
        d if d < 0 => "now".to_string(),
        0 => "now".to_string(),
        1..=59 => format!("{}s", diff),
        60..=3599 => format!("{}m", diff / 60),
        3600..=86399 => format!("{}h", diff / 3600),
        _ => {
            let dt = Local.timestamp_opt(ts, 0).unwrap();
            dt.format("%H:%M").to_string()
        }
    }
}

pub fn wrap_with_prefixes<'a>(
    content: &'a str,
    width: usize,
    prefix: &str,
    prefix_style: Style,
    content_style: Style,
) -> Vec<Line<'a>> {
    let mut lines = Vec::new();
    let prefix_display_width = UnicodeWidthStr::width(prefix);
    let available_content_width = width.saturating_sub(prefix_display_width);

    if content.is_empty() {
        lines.push(Line::from(vec![
            Span::styled(prefix.to_string(), prefix_style),
            Span::raw(""),
        ]));
        return lines;
    }

    let mut current_segment_start_byte_idx = 0;

    for (char_idx, char_val) in content.char_indices() {
        if char_val == '\n' {
            let segment = &content[current_segment_start_byte_idx..char_idx];
            let prefixed_line = Line::from(vec![
                Span::styled(prefix.to_string(), prefix_style),
                Span::styled(segment.to_string(), content_style),
            ]);
            lines.push(prefixed_line);
            current_segment_start_byte_idx = char_idx + char_val.len_utf8();
            continue;
        }

        let potential_segment =
            &content[current_segment_start_byte_idx..char_idx + char_val.len_utf8()];
        let potential_display_width = UnicodeWidthStr::width(potential_segment);

        if potential_display_width > available_content_width {
            let segment = &content[current_segment_start_byte_idx..char_idx];
            let prefixed_line = Line::from(vec![
                Span::styled(prefix.to_string(), prefix_style),
                Span::styled(segment.to_string(), content_style),
            ]);
            lines.push(prefixed_line);
            current_segment_start_byte_idx = char_idx;
        }
    }

    if current_segment_start_byte_idx < content.len() {
        let segment = &content[current_segment_start_byte_idx..];
        let prefixed_line = Line::from(vec![
            Span::styled(prefix.to_string(), prefix_style),
            Span::styled(segment.to_string(), content_style),
        ]);
        lines.push(prefixed_line);
    }

    lines
}
pub fn split_input_lines(input: &str, width: usize) -> Vec<String> {
    let mut lines = vec![];
    let mut current_byte_offset = 0;
    let mut current_line_display_width = 0;

    for (byte_idx, char_val) in input.char_indices() {
        let char_display_width = UnicodeWidthChar::width(char_val).unwrap_or(0);

        if current_line_display_width + char_display_width > width || char_val == '\n' {
            lines.push(input[current_byte_offset..byte_idx].to_string());

            current_byte_offset = byte_idx
                + if char_val == '\n' {
                    char_val.len_utf8()
                } else {
                    0
                };
            current_line_display_width = if char_val == '\n' {
                0
            } else {
                char_display_width
            };
        } else {
            current_line_display_width += char_display_width;
        }
    }
    lines.push(input[current_byte_offset..].to_string());

    if lines
        .last()
        .map_or(false, |l| l.is_empty() && input.ends_with('\n'))
    {
        lines.pop();
    }
    lines
}

pub fn cursor_line_col(cursor_byte_idx: usize, lines: &[String]) -> (usize, usize) {
    let mut current_byte_position = 0;
    for (line_idx, line_str) in lines.iter().enumerate() {
        let line_byte_len = line_str.len();
        if cursor_byte_idx >= current_byte_position
            && cursor_byte_idx <= current_byte_position + line_byte_len
        {
            let offset_in_line_bytes = cursor_byte_idx - current_byte_position;
            let col = line_str[..offset_in_line_bytes].width();
            return (line_idx, col);
        }
        current_byte_position += line_byte_len;
    }
    let last_line_idx = lines.len().saturating_sub(1);
    let last_line_visual_width = lines.last().map_or(0, |l| l.width());
    (last_line_idx, last_line_visual_width)
}
