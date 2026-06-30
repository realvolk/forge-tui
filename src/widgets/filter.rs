use crate::contract::Response;
use crate::theme::Theme;
use crate::widgets::helpers;
use anyhow::Result;
use crossterm::event::{self, Event, KeyCode};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Layout},
    widgets::{List, ListItem, ListState, Paragraph, Wrap},
    Terminal, Frame,
};
use std::fs::File;

pub fn run(
    terminal: Option<&mut Terminal<CrosstermBackend<File>>>,
    title: String,
    message: String,
    choices: Vec<String>,
    placeholder: Option<String>,
) -> Result<Response> {
    let is_daemon = terminal.is_some();
    let theme = Theme::load();
    let mut query = String::new();
    let mut list_state = ListState::default().with_selected(Some(0));

    let mut owned;
    let term: &mut Terminal<CrosstermBackend<File>> = match terminal {
        Some(t) => t,
        None => {
            owned = helpers::setup_one_shot()?;
            &mut owned
        }
    };

    let result = loop {
        let filtered: Vec<&String> = if query.is_empty() { choices.iter().collect() } else {
            let q = query.to_lowercase(); choices.iter().filter(|c| c.to_lowercase().contains(&q)).collect()
        };
        if let Some(sel) = list_state.selected() { if sel >= filtered.len() && !filtered.is_empty() { list_state.select(Some(filtered.len() - 1)); } }
        if filtered.is_empty() { list_state.select(None); }

        term.draw(|f: &mut Frame| {
            let area = f.size();
            if let Some(ref wm) = theme.watermark_path { crate::watermark::render(f, area, wm); }
            let inner = helpers::render_box(f, area, &title);

            let has_msg = !message.is_empty();
            let constraints: Vec<Constraint> = if has_msg {
                vec![Constraint::Length(2), Constraint::Length(3), Constraint::Min(1), Constraint::Length(1)]
            } else {
                vec![Constraint::Length(3), Constraint::Min(1), Constraint::Length(1)]
            };
            let chunks = Layout::default().constraints(constraints).split(inner);
            let footer_idx = chunks.len() - 1;

            let mut off = 0;
            if has_msg {
                f.render_widget(Paragraph::new(message.as_str()).style(theme.normal_style).wrap(Wrap { trim: false }), chunks[0]);
                off = 1;
            }

            let sd = if query.is_empty() { placeholder.as_deref().unwrap_or("Type to filter...") } else { &query };
            let ss = if query.is_empty() { theme.muted_style } else { theme.accent_style };
            f.render_widget(Paragraph::new(format!("> {}", sd)).style(ss), chunks[off]);
            f.set_cursor(chunks[off].x + 2 + query.len() as u16, chunks[off].y);

            let items: Vec<ListItem> = filtered.iter().map(|c| ListItem::new(c.as_str())).collect();
            f.render_stateful_widget(
                List::new(items).highlight_style(theme.selected_style).highlight_symbol(" >"),
                chunks[off + 1], &mut list_state.clone(),
            );

            f.render_widget(helpers::footer("Type:filter  j/k:move  Enter:select  Esc:cancel  Ctrl+C:quit"), chunks[footer_idx]);
        })?;

        match event::read()? {
            Event::Key(key) => {
                if helpers::is_cancel(&Event::Key(key)) {
                    break Response { result: None, cancelled: true, error: None };
                }
                match key.code {
                    KeyCode::Enter => {
                        if let Some(idx) = list_state.selected() {
                            if let Some(choice) = filtered.get(idx) {
                                break Response { result: Some(serde_json::Value::String(choice.to_string())), cancelled: false, error: None };
                            }
                        }
                    }
                    KeyCode::Up | KeyCode::Char('k') => { let i = list_state.selected().unwrap_or(0); if i > 0 { list_state.select(Some(i - 1)); } }
                    KeyCode::Down | KeyCode::Char('j') => { let i = list_state.selected().unwrap_or(0); if i < filtered.len().saturating_sub(1) { list_state.select(Some(i + 1)); } }
                    KeyCode::Char(c) => { query.push(c); list_state.select(Some(0)); }
                    KeyCode::Backspace => { query.pop(); list_state.select(Some(0)); }
                    _ => {}
                }
            }
            _ => {}
        }
    };

    if !is_daemon { helpers::teardown_one_shot()?; }
    Ok(result)
}