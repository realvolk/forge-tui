use crate::contract::Response;
use crate::layout;
use crate::theme::Theme;
use anyhow::Result;
use crossterm::event::{self, Event, KeyCode};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Layout, Margin, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph, Wrap},
    Terminal, Frame,
};
use std::collections::HashSet;
use std::fs::File;
use std::io;

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

    let mut owned_terminal;
    let terminal = match terminal {
        Some(t) => t,
        None => {
            let stdout = crate::tty::open()?;
            crossterm::terminal::enable_raw_mode()?;
            crossterm::execute!(io::stdout(), crossterm::terminal::EnterAlternateScreen)?;
            crossterm::execute!(io::stdout(), crossterm::cursor::Hide)?;
            owned_terminal = Terminal::new(CrosstermBackend::new(stdout))?;
            &mut owned_terminal
        }
    };

    let result = loop {
        let filtered: Vec<&String> = if query.is_empty() {
            choices.iter().collect()
        } else {
            let q = query.to_lowercase();
            choices.iter().filter(|c| c.to_lowercase().contains(&q)).collect()
        };

        // Clamp selection to filtered list
        if let Some(sel) = list_state.selected() {
            if sel >= filtered.len() && !filtered.is_empty() {
                list_state.select(Some(filtered.len() - 1));
            }
        }
        if filtered.is_empty() {
            list_state.select(None);
        }

        terminal.draw(|f: &mut Frame| {
            let area = f.size();
            if let Some(ref wm) = theme.watermark_path {
                crate::watermark::render(f, area, wm);
            }

            let box_area = layout::centered(70, 75, area);
            f.render_widget(Clear, box_area);

            let block = Block::default()
                .borders(Borders::ALL)
                .border_style(theme.border_style)
                .title(title.as_str())
                .title_style(theme.title_style);
            f.render_widget(block, box_area);

            let inner = box_area.inner(&Margin::new(2, 1));

            let has_msg = !message.is_empty();
            let constraints: Vec<Constraint> = if has_msg {
                vec![Constraint::Length(2), Constraint::Length(3), Constraint::Min(1), Constraint::Length(1)]
            } else {
                vec![Constraint::Length(3), Constraint::Min(1), Constraint::Length(1)]
            };
            let chunks = Layout::default().constraints(constraints).split(inner);

            let mut offset = 0;
            if has_msg {
                f.render_widget(
                    Paragraph::new(message.as_str())
                        .style(theme.normal_style)
                        .wrap(Wrap { trim: false }),
                    chunks[0],
                );
                offset = 1;
            }

            // Search bar
            let search_display = if query.is_empty() {
                placeholder.as_deref().unwrap_or("Type to filter...")
            } else {
                &query
            };
            let search_style = if query.is_empty() {
                theme.muted_style
            } else {
                theme.accent_style
            };
            f.render_widget(
                Paragraph::new(format!("> {}", search_display)).style(search_style),
                chunks[offset],
            );
            f.set_cursor(chunks[offset].x + 2 + query.len() as u16, chunks[offset].y);

            // Filtered list with checkboxes
            let list_idx = offset + 1;
            let items: Vec<ListItem> = filtered
                .iter()
                .enumerate()
                .map(|(i, c)| {
                    let original_idx = choices.iter().position(|x| x == *c).unwrap_or(i);
                    let mark = if selected.contains(&original_idx) { "[x]" } else { "[ ]" };
                    let is_sel = list_state.selected() == Some(i);
                    let style = if is_sel {
                        theme.selected_style
                    } else if selected.contains(&original_idx) {
                        theme.accent_style
                    } else {
                        theme.normal_style
                    };
                    ListItem::new(Line::from(Span::styled(
                        format!(" {} {}", mark, c),
                        style,
                    )))
                })
                .collect();

            f.render_stateful_widget(
                List::new(items)
                    .highlight_style(theme.selected_style)
                    .highlight_symbol(" >"),
                chunks[list_idx],
                &mut list_state.clone(),
            );

            // Status bar
            let status_idx = offset + 2;
            let status = format!(
                " Selected: {}/{}   Space=toggle  Enter=confirm  Esc=cancel",
                selected.len(),
                choices.len()
            );
            f.render_widget(
                Paragraph::new(status.as_str()).style(theme.muted_style),
                chunks[status_idx],
            );
        })?;

        match event::read()? {
            Event::Key(key) => match key.code {
                KeyCode::Enter => {
                    if selected.len() >= min_items && selected.len() <= max_items {
                        let result_choices: Vec<String> = selected
                            .iter()
                            .map(|&i| choices[i].clone())
                            .collect();
                        break Response {
                            result: Some(serde_json::Value::Array(
                                result_choices
                                    .into_iter()
                                    .map(serde_json::Value::String)
                                    .collect(),
                            )),
                            cancelled: false,
                            error: None,
                        };
                    }
                }
                KeyCode::Esc => {
                    break Response {
                        result: None,
                        cancelled: true,
                        error: None,
                    };
                }
                KeyCode::Up | KeyCode::Char('k') => {
                    let i = list_state.selected().unwrap_or(0);
                    if i > 0 {
                        list_state.select(Some(i - 1));
                    }
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    let i = list_state.selected().unwrap_or(0);
                    if i < filtered.len().saturating_sub(1) {
                        list_state.select(Some(i + 1));
                    }
                }
                KeyCode::Char(' ') => {
                    if let Some(sel) = list_state.selected() {
                        if let Some(c) = filtered.get(sel) {
                            if let Some(original_idx) = choices.iter().position(|x| x == *c) {
                                if selected.contains(&original_idx) {
                                    if selected.len() > min_items {
                                        selected.remove(&original_idx);
                                    }
                                } else if selected.len() < max_items {
                                    selected.insert(original_idx);
                                }
                            }
                        }
                    }
                }
                KeyCode::Char(c) => {
                    query.push(c);
                    list_state.select(Some(0));
                }
                KeyCode::Backspace => {
                    query.pop();
                    list_state.select(Some(0));
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