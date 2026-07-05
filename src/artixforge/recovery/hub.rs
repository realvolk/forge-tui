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
struct StatusItem {
    key: String,
    label: String,
    value: String,
    status: String, // "ok", "warn", "error", "none"
}

#[derive(Debug, Clone)]
struct StatusCategory {
    label: String,
    items: Vec<StatusItem>,
}

#[derive(Debug, Clone, PartialEq)]
enum RecoveryMode {
    Browsing,
    ConfirmRepair(String, String), // (repair_key, description)
}

struct RecoveryState {
    categories: Vec<StatusCategory>,
    cat_idx: usize,
    item_idx: usize,
    mode: RecoveryMode,
    values: HashMap<String, String>,
}

pub fn run(
    terminal: Option<&mut Terminal<CrosstermBackend<File>>>,
    title: String,
    status_json: Value,
    repairs: Vec<Value>,
) -> Result<Response> {
    let is_daemon = terminal.is_some();
    let theme = Theme::load();

    // Parse status categories
    let mut categories = Vec::new();
    let mut values = HashMap::new();

    if let Some(arr) = status_json.as_array() {
        for cat_val in arr {
            let label = cat_val["label"].as_str().unwrap_or("").to_string();
            let mut items = Vec::new();
            if let Some(items_arr) = cat_val["items"].as_array() {
                for item_val in items_arr {
                    let key = item_val["key"].as_str().unwrap_or("").to_string();
                    let item_label = item_val["label"].as_str().unwrap_or("").to_string();
                    let value = item_val["value"].as_str().unwrap_or("").to_string();
                    let status = item_val["status"].as_str().unwrap_or("ok").to_string();
                    values.insert(key.clone(), value.clone());
                    items.push(StatusItem { key, label: item_label, value, status });
                }
            }
            categories.push(StatusCategory { label, items });
        }
    }

    // Parse repair actions
    let repair_actions: Vec<(String, String)> = repairs
        .iter()
        .filter_map(|v| {
            let key = v["key"].as_str().unwrap_or("").to_string();
            let desc = v["label"].as_str().unwrap_or("").to_string();
            if key.is_empty() { None } else { Some((key, desc)) }
        })
        .collect();

    let mut state = RecoveryState {
        categories,
        cat_idx: 0,
        item_idx: 0,
        mode: RecoveryMode::Browsing,
        values,
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

        term.draw(|f: &mut Frame| {
            let area = f.size();
            if let Some(ref wm) = theme.watermark_path {
                crate::watermark::render(f, area, wm);
            }

            let box_area = layout::centered(95, 95, area);
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

            // Left panel: status categories with color-coded indicators
            let cat_items: Vec<ListItem> = state
                .categories
                .iter()
                .enumerate()
                .map(|(i, cat)| {
                    let is_sel = i == state.cat_idx;
                    let style = if is_sel { theme.selected_style } else { theme.normal_style };
                    // Count warnings/errors in this category
                    let warns = cat.items.iter().filter(|item| item.status == "warn").count();
                    let errs = cat.items.iter().filter(|item| item.status == "error").count();
                    let indicator = if errs > 0 {
                        Span::styled(format!(" [{}!] ", errs), Style::default().fg(ratatui::style::Color::Red))
                    } else if warns > 0 {
                        Span::styled(format!(" [{}~] ", warns), Style::default().fg(ratatui::style::Color::Yellow))
                    } else {
                        Span::styled(" [OK] ", Style::default().fg(ratatui::style::Color::Green))
                    };
                    ListItem::new(Line::from(vec![
                        indicator,
                        Span::styled(cat.label.clone(), style.add_modifier(Modifier::BOLD)),
                    ]))
                })
                .collect();

            let mut cat_list_state = ListState::default().with_selected(Some(state.cat_idx));
            f.render_stateful_widget(
                List::new(cat_items).highlight_style(theme.selected_style),
                panels[0],
                &mut cat_list_state,
            );

            // Right panel: items for selected category
            if !state.categories.is_empty() {
                let cat = &state.categories[state.cat_idx];
                let item_lines: Vec<Line> = cat
                    .items
                    .iter()
                    .enumerate()
                    .map(|(i, item)| {
                        let is_sel = i == state.item_idx && state.mode == RecoveryMode::Browsing;
                        let style = if is_sel { theme.selected_style } else { theme.normal_style };
                        let status_color = match item.status.as_str() {
                            "ok" => ratatui::style::Color::Green,
                            "warn" => ratatui::style::Color::Yellow,
                            "error" => ratatui::style::Color::Red,
                            _ => ratatui::style::Color::Gray,
                        };
                        let status_icon = match item.status.as_str() {
                            "ok" => "OK",
                            "warn" => "~ ",
                            "error" => "!!",
                            _ => "  ",
                        };
                        Line::from(vec![
                            Span::styled(format!("[{}] ", status_icon), Style::default().fg(status_color)),
                            Span::styled(format!("{}: ", item.label), style),
                            Span::styled(&item.value, theme.accent_style),
                        ])
                    })
                    .collect();
                f.render_widget(
                    Paragraph::new(item_lines).block(Block::default().borders(Borders::LEFT)),
                    panels[1],
                );
            }

            // Bottom bar: repair actions + quit
            let repair_text: Vec<String> = repair_actions
                .iter()
                .enumerate()
                .map(|(i, (_, desc))| format!("F{}:{}", i + 1, desc))
                .collect();
            let footer = format!(
                "{}   j/k:nav  Tab:switch  Esc:quit",
                repair_text.join("  ")
            );
            f.render_widget(
                Paragraph::new(Line::from(Span::styled(footer, theme.muted_style)))
                    .alignment(Alignment::Center),
                main_chunks[1],
            );
        })?;

        // Handle confirm dialog
        if let RecoveryMode::ConfirmRepair(ref key, ref desc) = state.mode {
            if let Event::Key(key_event) = event::read()? {
                match key_event.code {
                    KeyCode::Char('y') | KeyCode::Char('Y') => {
                        let repair_key = key.clone();
                        break Response {
                            result: Some(Value::String(repair_key)),
                            cancelled: false,
                            error: None,
                        };
                    }
                    KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
                        state.mode = RecoveryMode::Browsing;
                    }
                    _ => {}
                }
            }
            continue;
        }

        match event::read()? {
            Event::Key(key) => match key.code {
                KeyCode::Esc => {
                    break Response { result: None, cancelled: true, error: None };
                }
                KeyCode::Up | KeyCode::Char('k') => {
                    state.item_idx = state.item_idx.saturating_sub(1);
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    if !state.categories.is_empty() {
                        let cat = &state.categories[state.cat_idx];
                        if state.item_idx + 1 < cat.items.len() {
                            state.item_idx += 1;
                        }
                    }
                }
                KeyCode::Left | KeyCode::Char('h') => {
                    state.cat_idx = state.cat_idx.saturating_sub(1);
                    state.item_idx = 0;
                }
                KeyCode::Right | KeyCode::Char('l') | KeyCode::Tab => {
                    if state.cat_idx + 1 < state.categories.len() {
                        state.cat_idx += 1;
                        state.item_idx = 0;
                    }
                }
                KeyCode::F(f) if (f as usize) >= 1 && (f as usize) <= repair_actions.len() => {
                    let idx = f as usize - 1;
                    let (ref key, ref desc) = repair_actions[idx];
                    state.mode = RecoveryMode::ConfirmRepair(key.clone(), desc.clone());
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