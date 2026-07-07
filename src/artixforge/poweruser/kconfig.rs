use crate::theme::Theme;
use crate::widgets::{self, helpers};
use anyhow::Result;
use crossterm::event::{self, Event, KeyCode};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Layout, Margin},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph, Wrap},
    Terminal, Frame,
};
use std::collections::HashMap;
use std::fs;
use std::fs::File;
use std::io;
use std::path::Path;

#[derive(Debug, Clone)]
struct ConfigEntry {
    key: String,
    value: String,          // "y", "m", "n", or string value
    original_line: String,   // the whole line as it appears in .config
    line_idx: usize,
    description: String,     // preceding comment lines (help text)
}

#[derive(Debug, Clone, PartialEq)]
enum KconfigMode {
    Browsing,
    EditingString(usize), // index into filtered list
    ConfirmQuit,
}

struct KconfigState {
    entries: Vec<ConfigEntry>,
    filtered_indices: Vec<usize>,
    query: String,
    selected: usize,
    scroll: u16,
    mode: KconfigMode,
    dirty: bool,
    config_path: String,
    original_lines: Vec<String>, // complete file content for rewriting
}

pub fn run(term: &mut Terminal<CrosstermBackend<File>>, config_path: &str) -> Result<()> {
    let theme = Theme::load();
    let content = fs::read_to_string(config_path)
        .unwrap_or_else(|_| String::new());
    let lines: Vec<String> = content.lines().map(|l| l.to_string()).collect();

    // Parse entries
    let mut entries = Vec::new();
    let mut current_comment = String::new();

    for (i, line) in lines.iter().enumerate() {
        let trimmed = line.trim();
        if trimmed.starts_with("# CONFIG_") && trimmed.ends_with("is not set") {
            // disabled option
            let key = trimmed
                .trim_start_matches("# ")
                .trim_end_matches(" is not set")
                .to_string();
            entries.push(ConfigEntry {
                key,
                value: "n".to_string(),
                original_line: line.clone(),
                line_idx: i,
                description: current_comment.clone(),
            });
            current_comment.clear();
        } else if trimmed.starts_with("CONFIG_") && trimmed.contains('=') {
            let parts: Vec<&str> = trimmed.splitn(2, '=').collect();
            let key = parts[0].to_string();
            let val = parts[1].trim_matches('"').to_string();
            // normalize: if val is "y" or "m", keep as is, else it's a string
            let value = if val == "y" || val == "m" { val } else { val };
            entries.push(ConfigEntry {
                key,
                value,
                original_line: line.clone(),
                line_idx: i,
                description: current_comment.clone(),
            });
            current_comment.clear();
        } else if trimmed.starts_with('#') && !trimmed.starts_with("# end of") {
            // accumulate comment
            if !current_comment.is_empty() {
                current_comment.push('\n');
            }
            current_comment.push_str(trimmed.trim_start_matches("# "));
        } else {
            // other lines
            current_comment.clear();
        }
    }

    let filtered_indices: Vec<usize> = (0..entries.len()).collect();
    let state = KconfigState {
        entries,
        filtered_indices,
        query: String::new(),
        selected: 0,
        scroll: 0,
        mode: KconfigMode::Browsing,
        dirty: false,
        config_path: config_path.to_string(),
        original_lines: lines,
    };

    run_ui(term, state, theme)
}

fn run_ui(
    term: &mut Terminal<CrosstermBackend<File>>,
    mut state: KconfigState,
    theme: Theme,
) -> Result<()> {

    loop {
        // Update filtered list based on query
        let filtered: Vec<usize> = if state.query.is_empty() {
            (0..state.entries.len()).collect()
        } else {
            let q = state.query.to_lowercase();
            state.entries.iter().enumerate()
                .filter(|(_, e)| e.key.to_lowercase().contains(&q) || e.description.to_lowercase().contains(&q))
                .map(|(i, _)| i)
                .collect()
        };
        state.filtered_indices = filtered;
        if state.filtered_indices.is_empty() {
            state.selected = 0;
        } else {
            state.selected = state.selected.min(state.filtered_indices.len().saturating_sub(1));
        }

        term.draw(|f: &mut Frame| {
            let area = f.size();
            if let Some(ref wm) = theme.watermark_path {
                crate::watermark::render(f, area, wm);
            }

            let box_area = crate::layout::centered(95, 95, area);
            f.render_widget(Clear, box_area);
            let block = Block::default()
                .borders(Borders::ALL)
                .border_style(theme.border_style)
                .title("Kernel Configuration Editor")
                .title_style(theme.title_style);
            f.render_widget(block, box_area);

            let inner = box_area.inner(&Margin::new(1, 1));
            let chunks = Layout::default()
                .constraints([
                    Constraint::Length(3),   // search bar
                    Constraint::Min(1),      // entries
                    Constraint::Length(1),   // footer
                ])
                .split(inner);

            // Search bar
            let search_text = if state.query.is_empty() {
                "Type to search (Esc to clear)".to_string()
            } else {
                format!("> {}", state.query)
            };
            let search_style = if state.query.is_empty() { theme.muted_style } else { theme.accent_style };
            f.render_widget(Paragraph::new(search_text).style(search_style), chunks[0]);
            if state.mode == KconfigMode::Browsing && state.query.is_empty() {
                // no cursor needed
            } else {
                f.set_cursor(chunks[0].x + 2 + state.query.len() as u16, chunks[0].y);
            }

            // Entry list
            let visible_height = chunks[1].height as usize;
            let start = state.scroll as usize;
            let end = (start + visible_height).min(state.filtered_indices.len());
            let items: Vec<ListItem> = state.filtered_indices[start..end]
                .iter()
                .enumerate()
                .map(|(i, &idx)| {
                    let entry = &state.entries[idx];
                    let is_sel = i + start == state.selected && state.mode == KconfigMode::Browsing;
                    let style = if is_sel { theme.selected_style } else { theme.normal_style };
                    let val_style = match entry.value.as_str() {
                        "y" => Style::default().fg(Color::Green),
                        "m" => Style::default().fg(Color::Yellow),
                        "n" => Style::default().fg(Color::Red),
                        _ => theme.accent_style,
                    };
                    let desc_short = if entry.description.len() > 60 {
                        format!("{}...", &entry.description[..57])
                    } else {
                        entry.description.clone()
                    };
                    ListItem::new(Line::from(vec![
                        Span::styled(format!(" {}: ", entry.key), style),
                        Span::styled(format!("{}", entry.value), val_style),
                        Span::styled("  ", theme.muted_style),
                        Span::styled(desc_short, theme.muted_style),
                    ]))
                })
                .collect();
            f.render_stateful_widget(
                List::new(items)
                    .highlight_style(theme.selected_style)
                    .highlight_symbol("> "),
                chunks[1],
                &mut ListState::default().with_selected(Some(state.selected.saturating_sub(start))),
            );

            // Footer
            let footer_text = match &state.mode {
                KconfigMode::Browsing => "j/k:move  Space:toggle(y/m/n)  Enter:edit string  /:search  s:save  Esc:quit",
                KconfigMode::EditingString(_) => "Enter:confirm  Esc:cancel",
                KconfigMode::ConfirmQuit => "Y:save & quit  N:discard & quit  Esc:continue",
            };
            f.render_widget(
                Paragraph::new(Line::from(Span::styled(footer_text, theme.muted_style)))
                    .alignment(ratatui::layout::Alignment::Center),
                chunks[2],
            );
        })?;

        match &state.mode {
            KconfigMode::EditingString(entry_idx) => {
                let idx = state.filtered_indices[*entry_idx];
                let current_val = state.entries[idx].value.clone();
                let resp = widgets::input::run(
                    Some(term),
                    "Edit Value".into(),
                    format!("Enter value for {}:", state.entries[idx].key),
                    Some(current_val),
                    None,
                    None,
                )?;
                if !resp.cancelled {
                    if let Some(new_val) = resp.result.and_then(|v| v.as_str().map(String::from)) {
                        state.entries[idx].value = new_val;
                        state.dirty = true;
                    }
                }
                state.mode = KconfigMode::Browsing;
            }
            KconfigMode::ConfirmQuit => {
                if let Event::Key(key) = event::read()? {
                    match key.code {
                        KeyCode::Char('y') | KeyCode::Char('Y') => {
                            if state.dirty {
                                save_config(&state)?;
                            }
                            return Ok(());
                        }
                        KeyCode::Char('n') | KeyCode::Char('N') => {
                            return Ok(());
                        }
                        KeyCode::Esc => {
                            state.mode = KconfigMode::Browsing;
                        }
                        _ => {}
                    }
                }
            }
            KconfigMode::Browsing => {
                if let Event::Key(key) = event::read()? {
                    match key.code {
                        KeyCode::Esc => {
                            if state.dirty {
                                state.mode = KconfigMode::ConfirmQuit;
                            } else {
                                return Ok(());
                            }
                        }
                        KeyCode::Up | KeyCode::Char('k') => {
                            if state.selected > 0 {
                                state.selected -= 1;
                                if state.selected < state.scroll as usize {
                                    state.scroll = state.selected as u16;
                                }
                            }
                        }
                        KeyCode::Down | KeyCode::Char('j') => {
                            if state.selected + 1 < state.filtered_indices.len() {
                                state.selected += 1;
                                let visible_height = (term.size()?.height.saturating_sub(6)) as usize;
                                if state.selected >= state.scroll as usize + visible_height {
                                    state.scroll = (state.selected - visible_height + 1) as u16;
                                }
                            }
                        }
                        KeyCode::Char(' ') => {
                            if let Some(&idx) = state.filtered_indices.get(state.selected) {
                                let entry = &mut state.entries[idx];
                                entry.value = match entry.value.as_str() {
                                    "y" => "m".to_string(),
                                    "m" => "n".to_string(),
                                    "n" => "y".to_string(),
                                    _ => "y".to_string(), // strings toggle to 'y'?
                                };
                                state.dirty = true;
                            }
                        }
                        KeyCode::Enter => {
                            if let Some(&idx) = state.filtered_indices.get(state.selected) {
                                let entry = &state.entries[idx];
                                if entry.value != "y" && entry.value != "m" && entry.value != "n" {
                                    state.mode = KconfigMode::EditingString(state.selected);
                                }
                            }
                        }
                        KeyCode::Char('/') => {
                            // start search (already typing will filter)
                        }
                        KeyCode::Char(c) => {
                            // Filter input
                            if c != ' ' {
                                state.query.push(c);
                                state.selected = 0;
                                state.scroll = 0;
                            }
                        }
                        KeyCode::Backspace => {
                            state.query.pop();
                            state.selected = 0;
                            state.scroll = 0;
                        }
                        KeyCode::Char('s') | KeyCode::Char('S') => {
                            save_config(&state)?;
                            state.dirty = false;
                        }
                        _ => {}
                    }
                }
            }
        }
    }
}

fn save_config(state: &KconfigState) -> Result<()> {
    let mut new_lines = state.original_lines.clone();
    // Update lines for modified entries
    for entry in &state.entries {
        let new_line = if entry.value == "n" {
            format!("# {} is not set", entry.key)
        } else if entry.value == "y" || entry.value == "m" {
            format!("{}={}", entry.key, entry.value)
        } else {
            format!("{}=\"{}\"", entry.key, entry.value)
        };
        if new_line != entry.original_line {
            new_lines[entry.line_idx] = new_line;
        }
    }
    let content = new_lines.join("\n") + "\n";
    fs::write(&state.config_path, content)?;
    std::fs::write("/tmp/artix-kconfig-edited", "1").ok();
    Ok(())
}