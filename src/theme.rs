use ratatui::style::{Color, Modifier, Style};
use std::env;

pub struct Theme {
    pub accent_color: Color,
    pub title_style: Style,
    pub accent_style: Style,
    pub selected_style: Style,
    pub normal_style: Style,
    pub border_style: Style,
    pub muted_style: Style,
    pub watermark_path: Option<String>,
}

impl Theme {
    pub fn load() -> Self {
        let title_code = env::var("GUM_TITLE_COLOR")
            .ok()
            .and_then(|s| s.parse::<u8>().ok())
            .unwrap_or(212);
        let accent_code = env::var("GUM_ACCENT_COLOR")
            .ok()
            .and_then(|s| s.parse::<u8>().ok())
            .unwrap_or(34);

        let title_color = ansi256_to_color(title_code);
        let accent_color = ansi256_to_color(accent_code);

        let watermark_path = env::var("FORGE_TUI_WATERMARK").ok();

        Theme {
            accent_color,
            title_style: Style::default()
                .fg(title_color)
                .add_modifier(Modifier::BOLD),
            accent_style: Style::default().fg(accent_color),
            selected_style: Style::default().fg(accent_color).bg(Color::DarkGray),
            normal_style: Style::default().fg(Color::White).bg(Color::Black),
            border_style: Style::default().fg(Color::Gray),
            muted_style: Style::default().fg(Color::DarkGray),
            watermark_path,
        }
    }
}

fn ansi256_to_color(code: u8) -> Color {
    match code {
        212 => Color::Rgb(255, 135, 175),
        39 => Color::Rgb(0, 175, 255),
        245 => Color::Rgb(138, 138, 138),
        250 => Color::Rgb(188, 188, 188),
        3 => Color::Rgb(215, 175, 0),
        34 => Color::Rgb(175, 215, 0),
        117 => Color::Rgb(135, 215, 255),
        196 => Color::Rgb(255, 0, 0),
        255 => Color::Rgb(238, 238, 238),
        11 => Color::Rgb(255, 255, 0),
        1 => Color::Red,
        2 => Color::Green,
        4 => Color::Blue,
        5 => Color::Magenta,
        6 => Color::Cyan,
        7 => Color::Gray,
        8 => Color::DarkGray,
        9 => Color::LightRed,
        10 => Color::LightGreen,
        12 => Color::LightBlue,
        13 => Color::LightMagenta,
        14 => Color::LightCyan,
        15 => Color::White,
        232..=255 => {
            let v = ((code - 232) as f32 / 23.0 * 255.0) as u8;
            Color::Rgb(v, v, v)
        }
        16..=231 => {
            let idx = code - 16;
            let r = ((idx / 36) % 6) as f32 / 5.0 * 255.0;
            let g = ((idx / 6) % 6) as f32 / 5.0 * 255.0;
            let b = (idx % 6) as f32 / 5.0 * 255.0;
            Color::Rgb(r as u8, g as u8, b as u8)
        }
        _ => Color::White,
    }
}