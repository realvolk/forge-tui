use crate::contract::Response;
use crate::theme::Theme;
use crate::widgets::helpers;
use anyhow::Result;
use crossterm::event::{self, Event, KeyCode};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Layout},
    widgets::{Paragraph, Wrap},
    Terminal, Frame,
};
use std::fs::File;

pub fn run(
    terminal: Option<&mut Terminal<CrosstermBackend<File>>>,
    title: String,
    message: String,
    default: Option<String>,
    placeholder: Option<String>,
    validation: Option<String>,
) -> Result<Response> {
    run_with_background(terminal, title, message, default, placeholder, validation, None)
}

pub fn run_with_background(
    terminal: Option<&mut Terminal<CrosstermBackend<File>>>,
    title: String,
    message: String,
    default: Option<String>,
    placeholder: Option<String>,
    validation: Option<String>,
    background: Option<&dyn Fn(&mut Frame)>,
) -> Result<Response> {
    let is_daemon = terminal.is_some();
    let theme = Theme::load();
    let mut input = default.unwrap_or_default();
    let mut cursor = input.len();

    let mut owned;
    let term: &mut Terminal<CrosstermBackend<File>> = match terminal {
        Some(t) => t,
        None => {
            owned = helpers::setup_one_shot()?;
            &mut owned
        }
    };

    let result = loop {
        let display = if input.is_empty() { placeholder.as_deref().unwrap_or("") } else { &input };

        term.draw(|f: &mut Frame| {
            let area = f.size();
            if let Some(ref wm) = theme.watermark_path { crate::watermark::render(f, area, wm); }

            let inner = if background.is_some() {
                helpers::render_overlay(f, area, &title, 50, 30, background)
            } else {
                helpers::render_box(f, area, &title)
            };

            let has_msg = !message.is_empty();
            let constraints: Vec<Constraint> = if has_msg {
                vec![Constraint::Length(2), Constraint::Length(3), Constraint::Length(1)]
            } else {
                vec![Constraint::Length(3), Constraint::Length(1)]
            };
            let chunks = Layout::default().constraints(constraints).split(inner);
            let footer_idx = chunks.len() - 1;

            if has_msg {
                f.render_widget(
                    Paragraph::new(message.as_str()).style(theme.normal_style).wrap(Wrap { trim: false }),
                    chunks[0],
                );
            }
            let ic = if has_msg { chunks[1] } else { chunks[0] };
            let style = if input.is_empty() { theme.muted_style } else { theme.accent_style };
            f.render_widget(Paragraph::new(format!("> {}", display)).style(style), ic);
            f.set_cursor(ic.x + 2 + cursor as u16, ic.y);

            f.render_widget(helpers::footer("Type + Enter:confirm  Esc:cancel  Ctrl+C:quit"), chunks[footer_idx]);
        })?;

        match event::read()? {
            Event::Key(key) => {
                if helpers::is_cancel(&Event::Key(key)) {
                    break Response { result: None, cancelled: true, error: None };
                }
                match key.code {
                    KeyCode::Enter => {
                        if let Some(ref pattern) = validation {
                            if let Ok(re) = regex_lite::Regex::new(pattern) {
                                if !re.is_match(&input) { continue; }
                            }
                        }
                        break Response { result: Some(serde_json::Value::String(input)), cancelled: false, error: None };
                    }
                    KeyCode::Char(c) => { input.insert(cursor, c); cursor += 1; }
                    KeyCode::Backspace => { if cursor > 0 { input.remove(cursor - 1); cursor -= 1; } }
                    KeyCode::Delete => { if cursor < input.len() { input.remove(cursor); } }
                    KeyCode::Left => { if cursor > 0 { cursor -= 1; } }
                    KeyCode::Right => { if cursor < input.len() { cursor += 1; } }
                    KeyCode::Home => cursor = 0,
                    KeyCode::End => cursor = input.len(),
                    _ => {}
                }
            }
            _ => {}
        }
    };

    if !is_daemon { helpers::teardown_one_shot()?; }
    Ok(result)
}