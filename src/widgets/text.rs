use crate::contract::Response;
use crate::layout;
use crate::theme::Theme;
use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyModifiers};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Alignment, Constraint, Layout, Margin},
    style::Style,
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
    Terminal, Frame,
};
use std::fs;
use std::fs::File;
use std::io;

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

    let mut cursor_row: usize = 0;
    let mut cursor_col: usize = 0;
    let mut scroll: u16 = 0;
    let mut show_help: bool = true;
    let mut show_line_numbers: bool = true;

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
        cursor_row = cursor_row.min(lines.len().saturating_sub(1));
        cursor_col = cursor_col.min(lines[cursor_row].len());

        let (visible_height, max_scroll) = {
            let area = terminal.get_frame().size();
            let box_area = layout::centered(90, 90, area);
            let inner = box_area.inner(&Margin::new(1, 1));
            let help_height = if show_help { 2 } else { 0 };
            let chunks = Layout::default()
                .constraints([Constraint::Min(1), Constraint::Length(help_height)])
                .split(inner);
            let vh = chunks[0].height as usize;
            let ms = (lines.len().saturating_sub(vh)) as u16;
            (vh, ms)
        };

        terminal.draw(|f: &mut Frame| {
            let area = f.size();
            if let Some(ref wm) = theme.watermark_path {
                crate::watermark::render(f, area, wm);
            }

            let box_area = layout::centered(90, 90, area);
            f.render_widget(Clear, box_area);

            let block = Block::default()
                .borders(Borders::ALL)
                .border_style(theme.border_style)
                .title(title.as_str())
                .title_style(theme.title_style);
            f.render_widget(block, box_area);

            let inner = box_area.inner(&Margin::new(1, 1));
            let help_height = if show_help { 2 } else { 0 };
            let chunks = Layout::default()
                .constraints([Constraint::Min(1), Constraint::Length(help_height)])
                .split(inner);

            let line_number_width = if show_line_numbers {
                (lines.len().to_string().len() + 2) as u16
            } else {
                0
            };

            scroll = scroll.min(max_scroll);

            let display_lines: Vec<Line> = lines
                .iter()
                .enumerate()
                .skip(scroll as usize)
                .take(visible_height)
                .map(|(i, line)| {
                    let mut spans = Vec::new();
                    if show_line_numbers {
                        spans.push(Span::styled(
                            format!("{:>width$} ", i + 1, width = line_number_width as usize - 1),
                            theme.muted_style,
                        ));
                    }
                    spans.push(Span::styled(line.clone(), theme.normal_style));
                    Line::from(spans)
                })
                .collect();

            f.render_widget(Paragraph::new(display_lines), chunks[0]);

            if cursor_row >= scroll as usize && cursor_row < scroll as usize + visible_height {
                let x = if show_line_numbers { line_number_width } else { 0 } + cursor_col as u16;
                let y = (cursor_row - scroll as usize) as u16;
                f.set_cursor(chunks[0].x + x, chunks[0].y + y);
            }

            if show_help {
                f.render_widget(
                    Paragraph::new(Line::from(Span::styled(
                        " Ctrl+S:save  Esc:done  Ctrl+C:cancel  Ctrl+H:hide help  Ctrl+N:line nums  Arrows:move",
                        theme.muted_style,
                    )))
                    .alignment(Alignment::Center),
                    chunks[1],
                );
            }
        })?;

        match event::read()? {
            Event::Key(key) => match (key.code, key.modifiers) {
                (KeyCode::Char('s'), KeyModifiers::CONTROL) => {
                    let text = lines.join("\n");
                    if let Some(ref path) = file {
                        let _ = fs::write(path, &text);
                    }
                }
                (KeyCode::Char('c'), KeyModifiers::CONTROL) => {
                    break Response { result: None, cancelled: true, error: None };
                }
                (KeyCode::Char('h'), KeyModifiers::CONTROL) => {
                    show_help = !show_help;
                }
                (KeyCode::Char('n'), KeyModifiers::CONTROL) => {
                    show_line_numbers = !show_line_numbers;
                }
                (KeyCode::Esc, KeyModifiers::NONE) => {
                    let text = lines.join("\n");
                    break Response { result: Some(serde_json::Value::String(text)), cancelled: false, error: None };
                }
                (KeyCode::Enter, _) => {
                    let rest = lines[cursor_row].split_off(cursor_col);
                    lines.insert(cursor_row + 1, rest);
                    cursor_row += 1;
                    cursor_col = 0;
                }
                (KeyCode::Backspace, _) => {
                    if cursor_col > 0 {
                        lines[cursor_row].remove(cursor_col - 1);
                        cursor_col -= 1;
                    } else if cursor_row > 0 {
                        let current = lines.remove(cursor_row);
                        cursor_row -= 1;
                        cursor_col = lines[cursor_row].len();
                        lines[cursor_row].push_str(&current);
                    }
                }
                (KeyCode::Delete, _) => {
                    if cursor_col < lines[cursor_row].len() {
                        lines[cursor_row].remove(cursor_col);
                    } else if cursor_row + 1 < lines.len() {
                        let next = lines.remove(cursor_row + 1);
                        lines[cursor_row].push_str(&next);
                    }
                }
                (KeyCode::Left, _) => {
                    if cursor_col > 0 {
                        cursor_col -= 1;
                    } else if cursor_row > 0 {
                        cursor_row -= 1;
                        cursor_col = lines[cursor_row].len();
                    }
                }
                (KeyCode::Right, _) => {
                    if cursor_col < lines[cursor_row].len() {
                        cursor_col += 1;
                    } else if cursor_row + 1 < lines.len() {
                        cursor_row += 1;
                        cursor_col = 0;
                    }
                }
                (KeyCode::Up, _) => {
                    if cursor_row > 0 {
                        cursor_row -= 1;
                        cursor_col = cursor_col.min(lines[cursor_row].len());
                    }
                }
                (KeyCode::Down, _) => {
                    if cursor_row + 1 < lines.len() {
                        cursor_row += 1;
                        cursor_col = cursor_col.min(lines[cursor_row].len());
                    }
                }
                (KeyCode::Home, _) => cursor_col = 0,
                (KeyCode::End, _) => cursor_col = lines[cursor_row].len(),
                (KeyCode::PageUp, _) => {
                    cursor_row = cursor_row.saturating_sub(visible_height.saturating_sub(1));
                    scroll = scroll.saturating_sub(visible_height.saturating_sub(1) as u16);
                }
                (KeyCode::PageDown, _) => {
                    cursor_row = (cursor_row + visible_height.saturating_sub(1)).min(lines.len().saturating_sub(1));
                    scroll = (scroll + visible_height.saturating_sub(1) as u16).min(max_scroll);
                }
                (KeyCode::Home, KeyModifiers::CONTROL) => {
                    cursor_row = 0;
                    cursor_col = 0;
                    scroll = 0;
                }
                (KeyCode::End, KeyModifiers::CONTROL) => {
                    cursor_row = lines.len().saturating_sub(1);
                    cursor_col = lines[cursor_row].len();
                    scroll = max_scroll;
                }
                (KeyCode::Tab, _) => {
                    lines[cursor_row].insert_str(cursor_col, "    ");
                    cursor_col += 4;
                }
                (KeyCode::Char(c), _) => {
                    lines[cursor_row].insert(cursor_col, c);
                    cursor_col += 1;
                }
                _ => {}
            },
            Event::Mouse(mouse) => match mouse.kind {
                crossterm::event::MouseEventKind::ScrollDown => {
                    scroll = (scroll + 3).min(max_scroll);
                }
                crossterm::event::MouseEventKind::ScrollUp => {
                    scroll = scroll.saturating_sub(3);
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