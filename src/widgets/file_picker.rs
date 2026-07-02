use crate::contract::Response;
use crate::layout;
use crate::theme::Theme;
use crate::widgets::helpers;
use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, MouseEventKind};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Layout, Margin},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, ListState},
    Terminal, Frame,
};
use std::fs;
use std::fs::File;
use std::io;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq)]
enum PickerMode {
    Browsing,
    ConfirmQuit,
}

pub fn run(
    terminal: Option<&mut Terminal<CrosstermBackend<File>>>,
    title: String,
    start_dir: Option<String>,
    filter: Option<String>,
) -> Result<Response> {
    let is_daemon = terminal.is_some();
    let theme = Theme::load();

    let mut current_dir = start_dir
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("/"));
    let filter_ext = filter.unwrap_or_default();

    let mut entries: Vec<PathBuf> = get_entries(&current_dir, &filter_ext);
    let mut state = ListState::default().with_selected(Some(0));
    let mut mode = PickerMode::Browsing;

    let mut owned;
    let term: &mut Terminal<CrosstermBackend<File>> = match terminal {
        Some(t) => t,
        None => {
            let stdout = crate::tty::open()?;
            crossterm::terminal::enable_raw_mode()?;
            crossterm::execute!(io::stdout(), crossterm::terminal::EnterAlternateScreen)?;
            crossterm::execute!(io::stdout(), crossterm::event::EnableMouseCapture)?;
            crossterm::execute!(io::stdout(), crossterm::cursor::Hide)?;
            owned = Terminal::new(CrosstermBackend::new(stdout))?;
            &mut owned
        }
    };

    let result = loop {
        if entries.is_empty() {
            state.select(None);
        } else if state.selected().is_none() {
            state.select(Some(0));
        }

        term.draw(|f: &mut Frame| {
            let area = f.size();
            if let Some(ref wm) = theme.watermark_path {
                crate::watermark::render(f, area, wm);
            }

            let box_area = layout::centered(70, 75, area);
            f.render_widget(Clear, box_area);
            let block = Block::default()
                .borders(Borders::ALL)
                .border_style(theme.border_style)
                .title(format!("{}: {}", title, current_dir.display()))
                .title_style(theme.title_style);
            f.render_widget(block, box_area);

            let inner = box_area.inner(&Margin::new(2, 1));
            let chunks = Layout::default()
                .constraints([Constraint::Min(1), Constraint::Length(1)])
                .split(inner);

            let items: Vec<ListItem> = entries
                .iter()
                .enumerate()
                .map(|(i, entry)| {
                    let is_sel = state.selected() == Some(i);
                    let style = if is_sel {
                        theme.selected_style
                    } else {
                        theme.normal_style
                    };
                    let prefix = if entry.is_dir() { "[DIR]  " } else { "       " };
                    let name = entry.file_name().unwrap_or_default().to_string_lossy();
                    ListItem::new(Line::from(vec![Span::styled(
                        format!("{}{}", prefix, name),
                        style,
                    )]))
                })
                .collect();

            f.render_stateful_widget(
                List::new(items)
                    .highlight_style(theme.selected_style)
                    .highlight_symbol("> "),
                chunks[0],
                &mut state,
            );

            f.render_widget(
                helpers::footer("j/k:move  Enter:open/select  Esc:cancel  Backspace:parent"),
                chunks[1],
            );
        })?;

        if mode == PickerMode::ConfirmQuit {
            if let Event::Key(key) = event::read()? {
                match key.code {
                    KeyCode::Char('y') | KeyCode::Char('Y') => {
                        break Response {
                            result: None,
                            cancelled: true,
                            error: None,
                        };
                    }
                    _ => mode = PickerMode::Browsing,
                }
            }
            continue;
        }

        match event::read()? {
            Event::Key(key) => match key.code {
                KeyCode::Esc => mode = PickerMode::ConfirmQuit,
                KeyCode::Up | KeyCode::Char('k') => {
                    let i = state.selected().unwrap_or(0);
                    if i > 0 {
                        state.select(Some(i - 1));
                    }
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    let i = state.selected().unwrap_or(0);
                    if i < entries.len().saturating_sub(1) {
                        state.select(Some(i + 1));
                    }
                }
                KeyCode::Enter => {
                    if let Some(idx) = state.selected() {
                        if idx < entries.len() {
                            let entry = &entries[idx];
                            if entry.is_dir() {
                                current_dir = entry.clone();
                                entries = get_entries(&current_dir, &filter_ext);
                                state.select(Some(0));
                            } else {
                                break Response {
                                    result: Some(serde_json::Value::String(
                                        entry.to_string_lossy().to_string(),
                                    )),
                                    cancelled: false,
                                    error: None,
                                };
                            }
                        }
                    }
                }
                KeyCode::Backspace => {
                    if let Some(parent) = current_dir.parent() {
                        current_dir = parent.to_path_buf();
                        entries = get_entries(&current_dir, &filter_ext);
                        state.select(Some(0));
                    }
                }
                _ => {}
            },
            Event::Mouse(mouse) => match mouse.kind {
                MouseEventKind::ScrollDown => {
                    let i = state.selected().unwrap_or(0);
                    if i < entries.len().saturating_sub(1) {
                        state.select(Some(i + 1));
                    }
                }
                MouseEventKind::ScrollUp => {
                    let i = state.selected().unwrap_or(0);
                    if i > 0 {
                        state.select(Some(i - 1));
                    }
                }
                _ => {}
            },
            _ => {}
        }
    };

    if !is_daemon {
        crossterm::execute!(io::stdout(), crossterm::cursor::Show)?;
        crossterm::execute!(io::stdout(), crossterm::event::DisableMouseCapture)?;
        crossterm::execute!(io::stdout(), crossterm::terminal::LeaveAlternateScreen)?;
        crossterm::terminal::disable_raw_mode()?;
    }
    Ok(result)
}

fn get_entries(dir: &Path, filter: &str) -> Vec<PathBuf> {
    let mut entries: Vec<PathBuf> = if let Ok(read_dir) = fs::read_dir(dir) {
        read_dir
            .filter_map(|e| e.ok().map(|e| e.path()))
            .filter(|p| {
                if filter.is_empty() {
                    true
                } else if p.is_dir() {
                    true
                } else {
                    p.extension()
                        .map(|ext| ext.to_string_lossy() == filter)
                        .unwrap_or(false)
                }
            })
            .collect()
    } else {
        Vec::new()
    };
    entries.sort_by(|a, b| {
        let a_dir = a.is_dir();
        let b_dir = b.is_dir();
        if a_dir && !b_dir {
            std::cmp::Ordering::Less
        } else if !a_dir && b_dir {
            std::cmp::Ordering::Greater
        } else {
            a.file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .cmp(&b.file_name().unwrap_or_default().to_string_lossy())
        }
    });
    entries
}