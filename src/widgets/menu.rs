use crate::contract::Response;
use crate::theme::Theme;
use crate::widgets::helpers;
use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, MouseEventKind};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{List, ListItem, ListState, Paragraph, Wrap},
    Terminal, Frame,
};
use serde_json::Value;
use std::collections::HashMap;
use std::fs::File;

struct Choice {
    label: String,
    meta: Option<String>,
    stability: Option<String>,
    warning: Option<String>,
}

impl Choice {
    fn from_value(v: &Value) -> Self {
        match v {
            Value::String(s) => Choice { label: s.clone(), meta: None, stability: None, warning: None },
            Value::Object(obj) => Choice {
                label: obj.get("label").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                meta: obj.get("meta").and_then(|v| v.as_str()).map(String::from),
                stability: obj.get("stability").and_then(|v| v.as_str()).map(String::from),
                warning: obj.get("warning").and_then(|v| v.as_str()).map(String::from),
            },
            _ => Choice { label: String::new(), meta: None, stability: None, warning: None },
        }
    }
}

fn parse_color(name: &str) -> Color {
    match name.to_lowercase().as_str() {
        "red" => Color::Red, "green" => Color::Green, "yellow" => Color::Yellow,
        "blue" => Color::Blue, "magenta" => Color::Magenta, "cyan" => Color::Cyan,
        "white" => Color::White, "gray" | "grey" => Color::Gray, "darkgray" => Color::DarkGray,
        "lightred" => Color::LightRed, "lightgreen" => Color::LightGreen,
        "lightyellow" => Color::LightYellow, "lightblue" => Color::LightBlue,
        "lightmagenta" => Color::LightMagenta, "lightcyan" => Color::LightCyan,
        "black" => Color::Black, _ => Color::Gray,
    }
}

pub fn run(
    terminal: Option<&mut Terminal<CrosstermBackend<File>>>,
    title: String,
    message: String,
    choices_json: Value,
    default: Option<String>,
    stability_colors: Option<HashMap<String, String>>,
) -> Result<Response> {
    let is_daemon = terminal.is_some();
    let theme = Theme::load();

    let mut owned;
    let term: &mut Terminal<CrosstermBackend<File>> = match terminal {
        Some(t) => t,
        None => {
            helpers::enable_mouse()?;
            owned = helpers::setup_one_shot()?;
            &mut owned
        }
    };

    let choices: Vec<Choice> = match &choices_json {
        Value::Array(arr) => arr.iter().map(Choice::from_value).collect(),
        _ => vec![],
    };

    let default_idx = default.as_ref().and_then(|d| choices.iter().position(|c| c.label == *d)).unwrap_or(0);
    let mut state = ListState::default().with_selected(Some(default_idx));
    let stability_map = stability_colors.unwrap_or_default();
    let has_meta = choices.iter().any(|c| c.meta.is_some());
    let has_stability = choices.iter().any(|c| c.stability.is_some());
    let has_warnings = choices.iter().any(|c| c.warning.is_some());
    let has_info = has_warnings || has_meta;

    let result = loop {
        term.draw(|f: &mut Frame| {
            let area = f.size();
            if let Some(ref wm) = theme.watermark_path { crate::watermark::render(f, area, wm); }
            let inner = helpers::render_box(f, area, &title);

            let has_msg = !message.is_empty();
            let mut constraints: Vec<Constraint> = Vec::new();
            if has_msg { constraints.push(Constraint::Length(2)); }
            constraints.push(Constraint::Min(1));
            if has_info { constraints.push(Constraint::Length(3)); }
            constraints.push(Constraint::Length(1));
            let chunks = Layout::default().constraints(constraints).split(inner);
            let footer_idx = chunks.len() - 1;

            let mut offset = 0;
            if has_msg {
                f.render_widget(Paragraph::new(message.as_str()).style(theme.normal_style).wrap(Wrap { trim: false }), chunks[0]);
                offset = 1;
            }

            let items: Vec<ListItem> = choices.iter().enumerate().map(|(i, choice)| {
                let is_sel = state.selected() == Some(i);
                let mut spans: Vec<Span> = Vec::new();
                spans.push(Span::styled(if is_sel { "> " } else { "  " },
                    if is_sel { theme.accent_style } else { theme.normal_style }));
                let bs = if is_sel { theme.selected_style } else { theme.normal_style };
                spans.push(Span::styled(format!("{}  ", choice.label), bs));
                if has_meta {
                    spans.push(Span::styled(format!("{}  ", choice.meta.as_deref().unwrap_or("")),
                        if is_sel { theme.selected_style } else { theme.muted_style }));
                }
                if has_stability {
                    if let Some(ref st) = choice.stability {
                        let c = stability_map.get(st).map(|s| parse_color(s)).unwrap_or(Color::Gray);
                        spans.push(Span::styled("* ", Style::default().fg(c)));
                        spans.push(Span::styled(st.as_str(), Style::default().fg(c)));
                    }
                }
                ListItem::new(Line::from(spans))
            }).collect();
            f.render_stateful_widget(List::new(items), chunks[offset], &mut state.clone());

            if has_info {
                if let Some(idx) = state.selected() {
                    if let Some(choice) = choices.get(idx) {
                        let mut info: Vec<Span> = Vec::new();
                        if let Some(ref w) = choice.warning {
                            info.push(Span::styled("!  ", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)));
                            info.push(Span::styled(w.as_str(), Style::default().fg(Color::Yellow)));
                        } else if let Some(ref m) = choice.meta {
                            info.push(Span::styled(format!("{} -- {}", choice.label, m), theme.muted_style));
                        }
                        if !info.is_empty() { f.render_widget(Paragraph::new(Line::from(info)), chunks[offset + 1]); }
                    }
                }
            }

            f.render_widget(helpers::footer("j/k:move  Enter:select  Esc:cancel  Ctrl+C:quit"), chunks[footer_idx]);
        })?;

        match event::read()? {
            Event::Key(key) => {
                if helpers::is_cancel(&Event::Key(key)) {
                    break Response { result: None, cancelled: true, error: None };
                }
                match (key.code, key.modifiers) {
                    (KeyCode::Up, _) | (KeyCode::Char('k'), _) => {
                        if state.selected().unwrap_or(0) > 0 { state.select(Some(state.selected().unwrap_or(0) - 1)); }
                    }
                    (KeyCode::Down, _) | (KeyCode::Char('j'), _) => {
                        let i = state.selected().unwrap_or(0);
                        if i < choices.len().saturating_sub(1) { state.select(Some(i + 1)); }
                    }
                    (KeyCode::Enter, _) => {
                        let idx = state.selected().unwrap_or(default_idx);
                        break Response { result: Some(Value::String(choices[idx].label.clone())), cancelled: false, error: None };
                    }
                    _ => {}
                }
            }
            Event::Mouse(m) => match m.kind {
                MouseEventKind::ScrollDown => {
                    let i = state.selected().unwrap_or(0);
                    if i < choices.len().saturating_sub(1) { state.select(Some(i + 1)); }
                }
                MouseEventKind::ScrollUp => {
                    if state.selected().unwrap_or(0) > 0 { state.select(Some(state.selected().unwrap_or(0) - 1)); }
                }
                _ => {}
            },
            _ => {}
        }
    };

    if !is_daemon {
        helpers::disable_mouse()?;
        helpers::teardown_one_shot()?;
    }
    Ok(result)
}