use crate::contract::Response;
use crate::theme::Theme;
use crate::widgets::helpers;
use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, MouseEventKind};
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
    _height: Option<u16>,
    min: Option<usize>,
    max: Option<usize>,
    default: Option<Vec<String>>,
) -> Result<Response> {
    run_with_background(terminal, title, message, choices, _height, min, max, default, None)
}

pub fn run_with_background(
    terminal: Option<&mut Terminal<CrosstermBackend<File>>>,
    title: String,
    message: String,
    choices: Vec<String>,
    _height: Option<u16>,
    min: Option<usize>,
    max: Option<usize>,
    default: Option<Vec<String>>,
    background: Option<&dyn Fn(&mut Frame)>,
) -> Result<Response> {
    let is_daemon = terminal.is_some();
    let theme = Theme::load();
    let default_set: HashSet<String> = default.unwrap_or_default().into_iter().collect();
    let mut selected: HashSet<usize> = choices.iter().enumerate()
        .filter(|(_, c)| default_set.contains(*c)).map(|(i, _)| i).collect();
    let mut list_state = ListState::default().with_selected(Some(0));
    let min_items = min.unwrap_or(0);
    let max_items = max.unwrap_or(usize::MAX);

    let mut owned;
    let term: &mut Terminal<CrosstermBackend<File>> = match terminal {
        Some(t) => t,
        None => {
            helpers::enable_mouse()?;
            owned = helpers::setup_one_shot()?;
            &mut owned
        }
    };

    let result = loop {
        term.draw(|f: &mut Frame| {
            let area = f.size();
            if let Some(ref wm) = theme.watermark_path { crate::watermark::render(f, area, wm); }
            let width_pct = if choices.len() <= 5 { 50 } else { 70 };
            let height_pct = if choices.len() <= 5 { 50 } else { 75 };
            let inner = helpers::render_overlay(f, area, &title, width_pct, height_pct, background);

            let has_msg = !message.is_empty();
            let constraints: Vec<Constraint> = if has_msg {
                vec![Constraint::Length(2), Constraint::Min(1), Constraint::Length(1), Constraint::Length(1)]
            } else {
                vec![Constraint::Min(1), Constraint::Length(1), Constraint::Length(1)]
            };
            let chunks = Layout::default().constraints(constraints).split(inner);
            let footer_idx = chunks.len() - 1;
            let status_idx = footer_idx - 1;

            let mut off = 0;
            if has_msg {
                f.render_widget(Paragraph::new(message.as_str()).style(theme.normal_style).wrap(Wrap { trim: false }), chunks[0]);
                off = 1;
            }

            let items: Vec<ListItem> = choices.iter().enumerate().map(|(i, c)| {
                let mark = if selected.contains(&i) { "[x]" } else { "[ ]" };
                let style = if selected.contains(&i) { theme.accent_style } else { theme.normal_style };
                ListItem::new(format!(" {} {}", mark, c)).style(style)
            }).collect();
            f.render_stateful_widget(
                List::new(items).highlight_style(theme.selected_style).highlight_symbol(" >"),
                chunks[off], &mut list_state.clone(),
            );

            f.render_widget(
                Paragraph::new(format!(" Selected: {}/{}   Space=toggle  Enter=confirm  Esc=cancel  Ctrl+C:quit", selected.len(), choices.len()))
                    .style(theme.muted_style),
                chunks[status_idx],
            );
            f.render_widget(helpers::footer("j/k:move  Space:toggle  Enter:confirm  Esc:cancel  Ctrl+C:quit"), chunks[footer_idx]);
        })?;

        match event::read()? {
            Event::Key(key) => {
                if helpers::is_cancel(&Event::Key(key)) {
                    break Response { result: None, cancelled: true, error: None };
                }
                match key.code {
                    KeyCode::Up | KeyCode::Char('k') => {
                        let i = list_state.selected().unwrap_or(0);
                        if i > 0 { list_state.select(Some(i - 1)); }
                    }
                    KeyCode::Down | KeyCode::Char('j') => {
                        let i = list_state.selected().unwrap_or(0);
                        if i < choices.len().saturating_sub(1) { list_state.select(Some(i + 1)); }
                    }
                    KeyCode::Char(' ') => {
                        let i = list_state.selected().unwrap_or(0);
                        if selected.contains(&i) {
                            if selected.len() > min_items { selected.remove(&i); }
                        } else if selected.len() < max_items { selected.insert(i); }
                    }
                    KeyCode::Enter => {
                        if selected.len() >= min_items {
                            let rc: Vec<String> = selected.iter().map(|&i| choices[i].clone()).collect();
                            break Response { result: Some(serde_json::Value::Array(rc.into_iter().map(serde_json::Value::String).collect())), cancelled: false, error: None };
                        }
                    }
                    _ => {}
                }
            }
            Event::Mouse(m) => match m.kind {
                MouseEventKind::ScrollDown => {
                    let i = list_state.selected().unwrap_or(0);
                    if i < choices.len().saturating_sub(1) { list_state.select(Some(i + 1)); }
                }
                MouseEventKind::ScrollUp => {
                    let i = list_state.selected().unwrap_or(0);
                    if i > 0 { list_state.select(Some(i - 1)); }
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