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
struct HubItem {
    id: String,
    label: String,
    value: String,
    widget: String,
    choices: Vec<String>,
    placeholder: String,
    visible_if: HashMap<String, String>,
}

#[derive(Debug, Clone)]
struct HubCategory {
    label: String,
    items: Vec<HubItem>,
}

#[derive(Debug, Clone, PartialEq)]
enum HubMode {
    Browsing,
    EditingItem,
    ConfirmBuild,
}

struct HubState {
    categories: Vec<HubCategory>,
    cat_idx: usize,
    item_idx: usize,
    values: HashMap<String, String>,
    mode: HubMode,
}

pub fn run(
    terminal: Option<&mut Terminal<CrosstermBackend<File>>>,
    title: String,
    categories_json: Value,
) -> Result<Response> {
    let is_daemon = terminal.is_some();
    let theme = Theme::load();

    let mut categories = Vec::new();
    let mut initial_values = HashMap::new();

    if let Some(arr) = categories_json.as_array() {
        for cat_val in arr {
            let label = cat_val["label"].as_str().unwrap_or("").to_string();
            let mut items = Vec::new();
            if let Some(items_arr) = cat_val["items"].as_array() {
                for item_val in items_arr {
                    let id = item_val["id"].as_str().unwrap_or("").to_string();
                    let value = item_val["value"].as_str().unwrap_or("").to_string();
                    let visible_if: HashMap<String, String> = item_val
                        .get("visible_if")
                        .and_then(|v| v.as_object())
                        .map(|o| o.iter().map(|(k, v)| (k.clone(), v.as_str().unwrap_or("").to_string())).collect())
                        .unwrap_or_default();
                    initial_values.insert(id.clone(), value.clone());
                    items.push(HubItem {
                        id,
                        label: item_val["label"].as_str().unwrap_or("").to_string(),
                        value,
                        widget: item_val["widget"].as_str().unwrap_or("menu").to_string(),
                        choices: item_val["choices"].as_array().map(|a| a.iter().filter_map(|v| v.as_str().map(String::from)).collect()).unwrap_or_default(),
                        placeholder: item_val["placeholder"].as_str().unwrap_or("").to_string(),
                        visible_if,
                    });
                }
            }
            categories.push(HubCategory { label, items });
        }
    }

    let mut state = HubState {
        categories,
        cat_idx: 0,
        item_idx: 0,
        values: initial_values,
        mode: HubMode::Browsing,
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
        }

        let visible_items: Vec<&HubItem> = if !state.categories.is_empty() {
            let cat = &state.categories[state.cat_idx];
            cat.items.iter().filter(|item| {
                if item.visible_if.is_empty() { return true; }
                item.visible_if.iter().all(|(k, v)| state.values.get(k).map(|s| s == v).unwrap_or(false))
            }).collect()
        } else {
            vec![]
        };

        if !visible_items.is_empty() {
            state.item_idx = state.item_idx.min(visible_items.len() - 1);
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
                .constraints([Constraint::Percentage(35), Constraint::Percentage(65)])
                .split(main_chunks[0]);

            // Left: categories
            let cat_items: Vec<ListItem> = state.categories.iter().enumerate().map(|(i, cat)| {
                let is_sel = i == state.cat_idx;
                let style = if is_sel { theme.selected_style } else { theme.normal_style };
                ListItem::new(Line::from(Span::styled(cat.label.clone(), style.add_modifier(Modifier::BOLD))))
            }).collect();
            let mut cat_list = ListState::default().with_selected(Some(state.cat_idx));
            f.render_stateful_widget(List::new(cat_items).highlight_style(theme.selected_style), panels[0], &mut cat_list);

            // Right: items
            let item_lines: Vec<Line> = visible_items.iter().enumerate().map(|(i, item)| {
                let is_sel = i == state.item_idx && state.mode == HubMode::Browsing;
                let style = if is_sel { theme.selected_style } else { theme.normal_style };
                let val = state.values.get(&item.id).cloned().unwrap_or_default();
                Line::from(vec![
                    Span::styled(format!(" {}: ", item.label), style),
                    Span::styled(if val.is_empty() { "(not set)".into() } else { val }, theme.accent_style),
                ])
            }).collect();
            f.render_widget(Paragraph::new(item_lines).block(Block::default().borders(Borders::LEFT)), panels[1]);

            // Footer
            let footer = " j/k:nav  Tab:switch  Enter:edit  F1:Build  Esc:quit";
            f.render_widget(
                Paragraph::new(Line::from(Span::styled(footer, theme.muted_style))).alignment(Alignment::Center),
                main_chunks[1],
            );
        })?;

        // Editing mode
        if state.mode == HubMode::EditingItem {
            if let Some(item) = visible_items.get(state.item_idx).cloned() {
                let current_val = state.values.get(&item.id).cloned().unwrap_or_default();
                let result = match item.widget.as_str() {
                    "menu" => {
                        let resp = widgets::menu::run(Some(term), item.label.clone(), String::new(),
                            Value::Array(item.choices.iter().map(|c| Value::String(c.clone())).collect()),
                            Some(current_val), None)?;
                        if resp.cancelled { Ok::<Option<String>, anyhow::Error>(None) } else { Ok(resp.result.and_then(|v| v.as_str().map(String::from))) }
                    }
                    "input" => {
                        let resp = widgets::input::run(Some(term), item.label.clone(), String::new(),
                            Some(current_val), Some(item.placeholder.clone()), None)?;
                        if resp.cancelled { Ok::<Option<String>, anyhow::Error>(None) } else { Ok(resp.result.and_then(|v| v.as_str().map(String::from))) }
                    }
                    "yesno" => {
                        let default_yes = current_val == "yes";
                        let resp = widgets::yesno::run(Some(term), item.label.clone(), String::new(), Some(default_yes))?;
                        if resp.cancelled { Ok::<Option<String>, anyhow::Error>(None) } else {
                            Ok(resp.result.and_then(|v| v.as_bool()).map(|b| if b { "yes".to_string() } else { "no".to_string() }))
                        }
                    }
                    "checklist" => {
                        let resp = widgets::checklist::run(Some(term), item.label.clone(), String::new(),
                            item.choices.clone(), None, None, None, None)?;
                        if resp.cancelled { Ok::<Option<String>, anyhow::Error>(None) } else {
                            Ok(resp.result.and_then(|v| v.as_array().cloned())
                                .map(|a| a.iter().filter_map(|v| v.as_str()).collect::<Vec<_>>().join(" ")))
                        }
                    }
                    _ => Ok(None),
                };
                if let Ok(Some(new_val)) = result {
                    state.values.insert(item.id.clone(), new_val);
                }
            }
            state.mode = HubMode::Browsing;
            continue;
        }

        // Confirm build dialog
        if state.mode == HubMode::ConfirmBuild {
            if let Event::Key(key) = event::read()? {
                match key.code {
                    KeyCode::Char('y') | KeyCode::Char('Y') => {
                        let map: serde_json::Map<String, Value> = state.values.iter()
                            .map(|(k, v)| (k.clone(), Value::String(v.clone())))
                            .collect();
                        break Response { result: Some(Value::Object(map)), cancelled: false, error: None };
                    }
                    _ => state.mode = HubMode::Browsing,
                }
            }
            continue;
        }

        match event::read()? {
            Event::Key(key) => match key.code {
                KeyCode::Esc => break Response { result: None, cancelled: true, error: None },
                KeyCode::Up | KeyCode::Char('k') => { state.item_idx = state.item_idx.saturating_sub(1); }
                KeyCode::Down | KeyCode::Char('j') => {
                    if state.item_idx + 1 < visible_items.len() { state.item_idx += 1; }
                }
                KeyCode::Left | KeyCode::Char('h') => { state.cat_idx = state.cat_idx.saturating_sub(1); state.item_idx = 0; }
                KeyCode::Right | KeyCode::Char('l') | KeyCode::Tab => {
                    if state.cat_idx + 1 < state.categories.len() { state.cat_idx += 1; state.item_idx = 0; }
                }
                KeyCode::Enter => { state.mode = HubMode::EditingItem; }
                KeyCode::F(1) => { state.mode = HubMode::ConfirmBuild; }
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