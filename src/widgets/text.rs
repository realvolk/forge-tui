use crate::contract::Response;
use crate::theme::Theme;
use crate::widgets::helpers;
use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyModifiers};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Alignment, Constraint, Layout},
    text::{Line, Span},
    widgets::Paragraph,
    Terminal, Frame,
};
use std::fs;
use std::fs::File;

pub fn run(
    terminal: Option<&mut Terminal<CrosstermBackend<File>>>,
    title: String,
    file: Option<String>,
    content: Option<String>,
) -> Result<Response> {
    let is_daemon = terminal.is_some();
    let theme = Theme::load();

    let initial = if let Some(ref path) = file {
        fs::read_to_string(path).unwrap_or_else(|_| format!("[Error reading {}]", path))
    } else {
        content.unwrap_or_default()
    };

    let mut lines: Vec<String> = initial.lines().map(|l| l.to_string()).collect();
    if lines.is_empty() {
        lines.push(String::new());
    }
    let mut row: usize = 0;
    let mut col: usize = 0;
    let mut scroll: u16 = 0;
    let mut show_help = true;
    let mut show_nums = true;
    let mut vh: usize = 0;
    let mut ms: u16 = 0;

    let mut owned;
    let term: &mut Terminal<CrosstermBackend<File>> = match terminal {
        Some(t) => t,
        None => {
            owned = helpers::setup_one_shot()?;
            &mut owned
        }
    };

    let result = loop {
        row = row.min(lines.len().saturating_sub(1));
        col = col.min(lines[row].len());

        term.draw(|f: &mut Frame| {
            let area = f.size();
            if let Some(ref wm) = theme.watermark_path {
                crate::watermark::render(f, area, wm);
            }
            let inner = helpers::render_box(f, area, &title);

            let help_h = if show_help { 2 } else { 0 };
            let chunks = Layout::default()
                .constraints([Constraint::Min(1), Constraint::Length(help_h)])
                .split(inner);
            let num_w = if show_nums {
                (lines.len().to_string().len() + 2) as u16
            } else {
                0
            };

            vh = chunks[0].height as usize;
            ms = (lines.len().saturating_sub(vh)) as u16;
            scroll = scroll.min(ms);

            let display: Vec<Line> = lines
                .iter()
                .enumerate()
                .skip(scroll as usize)
                .take(vh)
                .map(|(i, l)| {
                    let mut s = Vec::new();
                    if show_nums {
                        s.push(Span::styled(
                            format!("{:>w$} ", i + 1, w = num_w as usize - 1),
                            theme.muted_style,
                        ));
                    }
                    s.push(Span::styled(l.clone(), theme.normal_style));
                    Line::from(s)
                })
                .collect();
            f.render_widget(Paragraph::new(display), chunks[0]);

            if row >= scroll as usize && row < scroll as usize + vh {
                let x = num_w + col as u16;
                let y = (row - scroll as usize) as u16;
                f.set_cursor(chunks[0].x + x, chunks[0].y + y);
            }

            if show_help {
                f.render_widget(
                    Paragraph::new(Line::from(Span::styled(
                        " Ctrl+S:save  Esc:done  Ctrl+C:cancel  Ctrl+H:hide help  Ctrl+N:line nums",
                        theme.muted_style,
                    )))
                    .alignment(Alignment::Center),
                    chunks[1],
                );
            }
        })?;

        match event::read()? {
            Event::Key(key) => match (key.code, key.modifiers) {
                (KeyCode::Char('c'), KeyModifiers::CONTROL) => {
                    break Response {
                        result: None,
                        cancelled: true,
                        error: None,
                    };
                }
                (KeyCode::Char('s'), KeyModifiers::CONTROL) => {
                    if let Some(ref path) = file {
                        let _ = fs::write(path, lines.join("\n"));
                    }
                }
                (KeyCode::Char('h'), KeyModifiers::CONTROL) => show_help = !show_help,
                (KeyCode::Char('n'), KeyModifiers::CONTROL) => show_nums = !show_nums,
                (KeyCode::Esc, _) => {
                    break Response {
                        result: Some(serde_json::Value::String(lines.join("\n"))),
                        cancelled: false,
                        error: None,
                    };
                }
                (KeyCode::Enter, _) => {
                    let rest = lines[row].split_off(col);
                    lines.insert(row + 1, rest);
                    row += 1;
                    col = 0;
                }
                (KeyCode::Backspace, _) => {
                    if col > 0 {
                        lines[row].remove(col - 1);
                        col -= 1;
                    } else if row > 0 {
                        let cur = lines.remove(row);
                        row -= 1;
                        col = lines[row].len();
                        lines[row].push_str(&cur);
                    }
                }
                (KeyCode::Delete, _) => {
                    if col < lines[row].len() {
                        lines[row].remove(col);
                    } else if row + 1 < lines.len() {
                        let next = lines.remove(row + 1);
                        lines[row].push_str(&next);
                    }
                }
                (KeyCode::Left, _) => {
                    if col > 0 {
                        col -= 1;
                    } else if row > 0 {
                        row -= 1;
                        col = lines[row].len();
                    }
                }
                (KeyCode::Right, _) => {
                    if col < lines[row].len() {
                        col += 1;
                    } else if row + 1 < lines.len() {
                        row += 1;
                        col = 0;
                    }
                }
                (KeyCode::Up, _) => {
                    if row > 0 {
                        row -= 1;
                        col = col.min(lines[row].len());
                    }
                }
                (KeyCode::Down, _) => {
                    if row + 1 < lines.len() {
                        row += 1;
                        col = col.min(lines[row].len());
                    }
                }
                (KeyCode::Home, KeyModifiers::NONE) => col = 0,
                (KeyCode::End, KeyModifiers::NONE) => col = lines[row].len(),
                (KeyCode::PageUp, _) => {
                    row = row.saturating_sub(vh.saturating_sub(1));
                    scroll = scroll.saturating_sub(vh.saturating_sub(1) as u16);
                }
                (KeyCode::PageDown, _) => {
                    row = (row + vh.saturating_sub(1)).min(lines.len().saturating_sub(1));
                    scroll = (scroll + vh.saturating_sub(1) as u16).min(ms);
                }
                (KeyCode::Home, KeyModifiers::CONTROL) => {
                    row = 0;
                    col = 0;
                    scroll = 0;
                }
                (KeyCode::End, KeyModifiers::CONTROL) => {
                    row = lines.len().saturating_sub(1);
                    col = lines[row].len();
                    scroll = ms;
                }
                (KeyCode::Tab, _) => {
                    lines[row].insert_str(col, "    ");
                    col += 4;
                }
                (KeyCode::Char(c), _) => {
                    lines[row].insert(col, c);
                    col += 1;
                }
                _ => {}
            },
            _ => {}
        }
    };

    if !is_daemon {
        helpers::teardown_one_shot()?;
    }
    Ok(result)
}