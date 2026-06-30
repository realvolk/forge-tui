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
use std::collections::HashSet;
use std::fs::File;

pub fn run(
    terminal: Option<&mut Terminal<CrosstermBackend<File>>>,
    title: String,
    message: String,
    choices: Vec<String>,
    placeholder: Option<String>,
    min: Option<usize>,
    max: Option<usize>,
) -> Result<Response> {
    let is_daemon = terminal.is_some();
    let theme = Theme::load();
    let mut query = String::new();
    let mut selected: HashSet<usize> = HashSet::new();
    let mut list_state = ListState::default().with_selected(Some(0));
    let min_items = min.unwrap_or(0);
    let max_items = max.unwrap_or(usize::MAX);

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

            let items: Vec<ListItem> = filtered.iter().enumerate().map(|(i, c)| {
                let orig_idx = choices.iter().position(|x| x == *c).unwrap_or(i);
                let mark = if selected.contains(&orig_idx) { "[x]" } else { "[ ]" };
                let is_sel = list_state.selected() == Some(i);
                let style = if is_sel { theme.selected_style } else if selected.contains(&orig_idx) { theme.accent_style } else { theme.normal_style };
                ListItem::new(format!(" {} {}", mark, c)).style(style)
            }).collect();
            f.render_stateful_widget(
                List::new(items).highlight_style(theme.selected_style).highlight_symbol(" >"),
                chunks[off + 1], &mut list_state.clone(),
            );

            f.render_widget(
                Paragraph::new(format!(" Selected: {}/{}   Space=toggle  Enter=confirm  Esc=cancel  Ctrl+C:quit", selected.len(), choices.len()))
                    .style(theme.muted_style),
                chunks[footer_idx - 1],
            );
            f.render_widget(helpers::footer("Type:filter  j/k:move  Space:toggle  Enter:confirm  Esc:cancel  Ctrl+C:quit"), chunks[footer_idx]);
        })?;

        match event::read()? {
            Event::Key(key) => {
                if helpers::is_cancel(&Event::Key(key)) {
                    break Response { result: None, cancelled: true, error: None };
                }
                match key.code {
                    KeyCode::Enter => {
                        if selected.len() >= min_items && selected.len() <= max_items {
                            let rc: Vec<String> = selected.iter().map(|&i| choices[i].clone()).collect();
                            break Response { result: Some(serde_json::Value::Array(rc.into_iter().map(serde_json::Value::String).collect())), cancelled: false, error: None };
                        }
                    }
                    KeyCode::Up | KeyCode::Char('k') => { let i = list_state.selected().unwrap_or(0); if i > 0 { list_state.select(Some(i - 1)); } }
                    KeyCode::Down | KeyCode::Char('j') => { let i = list_state.selected().unwrap_or(0); if i < filtered.len().saturating_sub(1) { list_state.select(Some(i + 1)); } }
                    KeyCode::Char(' ') => {
                        if let Some(sel) = list_state.selected() {
                            if let Some(c) = filtered.get(sel) {
                                if let Some(orig) = choices.iter().position(|x| x == *c) {
                                    if selected.contains(&orig) { if selected.len() > min_items { selected.remove(&orig); } }
                                    else if selected.len() < max_items { selected.insert(orig); }
                                }
                            }
                        }
                    }
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