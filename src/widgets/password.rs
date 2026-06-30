use crate::contract::Response;
use crate::theme::Theme;
use crate::widgets::helpers;
use anyhow::Result;
use crossterm::event::{self, Event, KeyCode};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Layout},
    style::{Color, Modifier, Style},
    text::Span,
    widgets::{Gauge, Paragraph, Wrap},
    Terminal, Frame,
};
use std::fs::File;

pub fn run(
    terminal: Option<&mut Terminal<CrosstermBackend<File>>>,
    title: String,
    message: String,
    placeholder: Option<String>,
) -> Result<Response> {
    let is_daemon = terminal.is_some();
    let theme = Theme::load();
    let mut password = String::new();

    let mut owned;
    let term: &mut Terminal<CrosstermBackend<File>> = match terminal {
        Some(t) => t,
        None => {
            owned = helpers::setup_one_shot()?;
            &mut owned
        }
    };

    let result = loop {
        let masked = "*".repeat(password.len());
        let display = if masked.is_empty() { placeholder.as_deref().unwrap_or("") } else { &masked };
        let (score, label, bar_color) = password_strength(&password);

        term.draw(|f: &mut Frame| {
            let area = f.size();
            if let Some(ref wm) = theme.watermark_path { crate::watermark::render(f, area, wm); }
            let inner = helpers::render_box(f, area, &title);

            let has_msg = !message.is_empty();
            let mut constraints: Vec<Constraint> = vec![];
            if has_msg { constraints.push(Constraint::Length(2)); }
            constraints.push(Constraint::Length(3));
            if !password.is_empty() { constraints.push(Constraint::Length(3)); }
            constraints.push(Constraint::Length(1));
            let chunks = Layout::default().constraints(constraints).split(inner);
            let footer_idx = chunks.len() - 1;

            let mut off = 0;
            if has_msg {
                f.render_widget(Paragraph::new(message.as_str()).style(theme.normal_style).wrap(Wrap { trim: false }), chunks[0]);
                off = 1;
            }
            let ic = chunks[off];
            let style = if password.is_empty() { theme.muted_style } else { theme.accent_style };
            f.render_widget(Paragraph::new(format!("> {}", display)).style(style), ic);

            if !password.is_empty() {
                f.render_widget(
                    Gauge::default()
                        .gauge_style(Style::default().fg(bar_color).add_modifier(Modifier::BOLD))
                        .percent(score as u16)
                        .label(label),
                    chunks[off + 1],
                );
            }

            f.render_widget(helpers::footer("Type + Enter:confirm  Esc:cancel  Ctrl+C:quit"), chunks[footer_idx]);
        })?;

        match event::read()? {
            Event::Key(key) => {
                if helpers::is_cancel(&Event::Key(key)) {
                    break Response { result: None, cancelled: true, error: None };
                }
                match key.code {
                    KeyCode::Enter => break Response { result: Some(serde_json::Value::String(password)), cancelled: false, error: None },
                    KeyCode::Char(c) => password.push(c),
                    KeyCode::Backspace => { password.pop(); }
                    _ => {}
                }
            }
            _ => {}
        }
    };

    if !is_daemon { helpers::teardown_one_shot()?; }
    Ok(result)
}

fn password_strength(password: &str) -> (u8, &'static str, Color) {
    let len = password.len();
    if len == 0 { return (0, "Enter password", Color::Gray); }
    let has_upper = password.chars().any(|c| c.is_uppercase());
    let has_lower = password.chars().any(|c| c.is_lowercase());
    let has_digit = password.chars().any(|c| c.is_ascii_digit());
    let has_special = password.chars().any(|c| !c.is_alphanumeric());
    let mut score: u8 = (len.min(16) * 2) as u8;
    if has_upper { score += 15; }
    if has_lower { score += 15; }
    if has_digit { score += 15; }
    if has_special { score += 15; }
    if has_upper && has_lower { score += 10; }
    if len >= 12 { score += 10; }
    if len >= 16 { score += 10; }
    score = score.min(100);
    let (label, color) = match score {
        0..=25 => ("Weak", Color::Red),
        26..=50 => ("Fair", Color::Yellow),
        51..=75 => ("Good", Color::Blue),
        _ => ("Strong", Color::Green),
    };
    (score, label, color)
}