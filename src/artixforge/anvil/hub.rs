use crate::contract::Response;
use crate::layout;
use crate::theme::Theme;
use crate::widgets;
use anyhow::Result;
use crossterm::event::{self, Event, KeyCode};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Alignment, Constraint, Layout, Margin},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph},
    Terminal, Frame,
};
use serde_json::Value;
use std::collections::HashMap;
use std::fs::File;
use std::io;

#[derive(Debug, Clone)]
struct Action {
    key: String,
    label: String,
    description: String,
}

#[derive(Debug, Clone, PartialEq)]
enum AnvilMode {
    Browsing,
    ConfirmAction(String),
    ShowResult(String, String), // (title, message)
}

struct AnvilState {
    categories: Vec<(String, Vec<Action>)>, // (category_name, actions)
    cat_idx: usize,
    action_idx: usize,
    mode: AnvilMode,
}

pub fn run(
    terminal: Option<&mut Terminal<CrosstermBackend<File>>>,
    title: String,
    actions_json: Value,
) -> Result<Response> {
    let is_daemon = terminal.is_some();
    let theme = Theme::load();

    let mut categories: Vec<(String, Vec<Action>)> = Vec::new();

    if let Some(arr) = actions_json.as_array() {
        for cat_val in arr {
            let cat_name = cat_val["category"].as_str().unwrap_or("").to_string();
            let mut actions = Vec::new();
            if let Some(items) = cat_val["actions"].as_array() {
                for item in items {
                    actions.push(Action {
                        key: item["key"].as_str().unwrap_or("").to_string(),
                        label: item["label"].as_str().unwrap_or("").to_string(),
                        description: item["description"].as_str().unwrap_or("").to_string(),
                    });
                }
            }
            if !actions.is_empty() {
                categories.push((cat_name, actions));
            }
        }
    }

    let mut state = AnvilState {
        categories,
        cat_idx: 0,
        action_idx: 0,
        mode: AnvilMode::Browsing,
    };

    let mut owned;
    let term: &mut Terminal<CrosstermBackend<File>> = match terminal {
        Some(t) => t,
        None => {
            let stdout = crate::tty::open()?;
            crossterm::terminal::enable_raw_mode()?;
            crossterm::execute!(io::stdout(), crossterm::terminal::EnterAlternateScreen)?;
            crossterm::execute!(io::stdout(), crossterm::cursor::Hide)?;
            owned = Terminal::new(CrosstermBackend::new(stdout))?;
            &mut owned
        }
    };

    let result = loop {
        if !state.categories.is_empty() {
            state.cat_idx = state.cat_idx.min(state.categories.len() - 1);
            let actions = &state.categories[state.cat_idx].1;
            if !actions.is_empty() {
                state.action_idx = state.action_idx.min(actions.len() - 1);
            }
        }

        term.draw(|f: &mut Frame| {
            let area = f.size();
            if let Some(ref wm) = theme.watermark_path {
                crate::watermark::render(f, area, wm);
            }

            let box_area = layout::centered(85, 90, area);
            f.render_widget(Clear, box_area);
            let block = Block::default()
                .borders(Borders::ALL)
                .border_style(theme.border_style)
                .title(title.as_str())
                .title_style(theme.title_style);
            f.render_widget(block, box_area);

            let inner = box_area.inner(&Margin::new(1, 1));
            let main_chunks = Layout::default()
                .constraints([Constraint::Min(1), Constraint::Length(1)])
                .split(inner);

            let panels = Layout::default()
                .constraints([Constraint::Percentage(30), Constraint::Percentage(70)])
                .split(main_chunks[0]);

            // Left: categories
            let cat_items: Vec<ListItem> = state.categories.iter().enumerate().map(|(i, (name, _))| {
                let is_sel = i == state.cat_idx;
                let style = if is_sel { theme.selected_style } else { theme.normal_style };
                ListItem::new(Line::from(Span::styled(name.clone(), style.add_modifier(Modifier::BOLD))))
            }).collect();
            let mut cat_list = ListState::default().with_selected(Some(state.cat_idx));
            f.render_stateful_widget(List::new(cat_items).highlight_style(theme.selected_style), panels[0], &mut cat_list);

            // Right: actions for selected category
            if !state.categories.is_empty() {
                let actions = &state.categories[state.cat_idx].1;
                let action_lines: Vec<Line> = actions.iter().enumerate().map(|(i, action)| {
                    let is_sel = i == state.action_idx && state.mode == AnvilMode::Browsing;
                    let style = if is_sel { theme.selected_style } else { theme.normal_style };
                    Line::from(vec![
                        Span::styled(action.label.clone(), style),
                        Span::styled(format!("  — {}", action.description), theme.muted_style),
                    ])
                }).collect();
                f.render_widget(Paragraph::new(action_lines).block(Block::default().borders(Borders::LEFT)), panels[1]);
            }

            // Footer
            let footer = match &state.mode {
                AnvilMode::Browsing => " j/k:nav  Tab:switch  Enter:execute  q:quit",
                AnvilMode::ConfirmAction(_) => " [Y]es  [N]o",
                AnvilMode::ShowResult(_, _) => " Any key:continue",
            };
            f.render_widget(
                Paragraph::new(Line::from(Span::styled(footer, theme.muted_style))).alignment(Alignment::Center),
                main_chunks[1],
            );
        })?;

        // Handle sub-modes
        match &state.mode {
            AnvilMode::ShowResult(_, _) => {
                if let Event::Key(_) = event::read()? {
                    state.mode = AnvilMode::Browsing;
                }
                continue;
            }
            AnvilMode::ConfirmAction(key) => {
                let key = key.clone();
                if let Event::Key(key_event) = event::read()? {
                    match key_event.code {
                        KeyCode::Char('y') | KeyCode::Char('Y') => {
                            break Response { result: Some(Value::String(key)), cancelled: false, error: None };
                        }
                        _ => state.mode = AnvilMode::Browsing,
                    }
                }
                continue;
            }
            AnvilMode::Browsing => {}
        }

        match event::read()? {
            Event::Key(key) => match key.code {
                KeyCode::Esc | KeyCode::Char('q') => {
                    break Response { result: None, cancelled: true, error: None };
                }
                KeyCode::Up | KeyCode::Char('k') => { state.action_idx = state.action_idx.saturating_sub(1); }
                KeyCode::Down | KeyCode::Char('j') => {
                    if !state.categories.is_empty() {
                        let actions = &state.categories[state.cat_idx].1;
                        if state.action_idx + 1 < actions.len() { state.action_idx += 1; }
                    }
                }
                KeyCode::Left | KeyCode::Char('h') => { state.cat_idx = state.cat_idx.saturating_sub(1); state.action_idx = 0; }
                KeyCode::Right | KeyCode::Char('l') | KeyCode::Tab => {
                    if state.cat_idx + 1 < state.categories.len() { state.cat_idx += 1; state.action_idx = 0; }
                }
                KeyCode::Enter => {
                    if !state.categories.is_empty() {
                        let actions = &state.categories[state.cat_idx].1;
                        if state.action_idx < actions.len() {
                            let key = actions[state.action_idx].key.clone();
                            state.mode = AnvilMode::ConfirmAction(key);
                        }
                    }
                }
                _ => {}
            },
            _ => {}
        }
    };

    if !is_daemon {
        crossterm::execute!(io::stdout(), crossterm::cursor::Show)?;
        crossterm::execute!(io::stdout(), crossterm::terminal::LeaveAlternateScreen)?;
        crossterm::terminal::disable_raw_mode()?;
    }
    Ok(result)
}