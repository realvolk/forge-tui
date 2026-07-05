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

pub fn run(
    terminal: Option<&mut Terminal<CrosstermBackend<File>>>,
    title: String,
    current_init: String,
) -> Result<Response> {
    let is_daemon = terminal.is_some();
    let theme = Theme::load();

    let source_inits = vec!["openrc", "runit", "dinit", "s6", "systemd"];
    let target_inits = vec!["openrc", "runit", "dinit", "s6"];

    let mut source_idx = source_inits.iter().position(|i| i == &current_init).unwrap_or(0);
    let mut target_idx = 0;
    let mut field: usize = 0; // 0 = source, 1 = target

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
        term.draw(|f: &mut Frame| {
            let area = f.size();
            if let Some(ref wm) = theme.watermark_path {
                crate::watermark::render(f, area, wm);
            }

            let box_area = layout::centered(50, 35, area);
            f.render_widget(Clear, box_area);
            let block = Block::default()
                .borders(Borders::ALL)
                .border_style(theme.border_style)
                .title(title.as_str())
                .title_style(theme.title_style);
            f.render_widget(block, box_area);

            let inner = box_area.inner(&Margin::new(2, 1));
            let chunks = Layout::default()
                .constraints([Constraint::Length(3), Constraint::Length(3), Constraint::Length(1)])
                .split(inner);

            let source_style = if field == 0 { theme.selected_style } else { theme.normal_style };
            f.render_widget(
                Paragraph::new(Line::from(vec![
                    Span::styled("Source: ", theme.muted_style),
                    Span::styled(source_inits[source_idx], source_style),
                ])),
                chunks[0],
            );

            let target_style = if field == 1 { theme.selected_style } else { theme.normal_style };
            f.render_widget(
                Paragraph::new(Line::from(vec![
                    Span::styled("Target: ", theme.muted_style),
                    Span::styled(target_inits[target_idx], target_style),
                ])),
                chunks[1],
            );

            f.render_widget(
                Paragraph::new(Line::from(Span::styled(
                    " j/k:field  h/l:change  Enter:confirm  Esc:cancel",
                    theme.muted_style,
                ))).alignment(Alignment::Center),
                chunks[2],
            );
        })?;

        match event::read()? {
            Event::Key(key) => match key.code {
                KeyCode::Esc => break Response { result: None, cancelled: true, error: None },
                KeyCode::Up | KeyCode::Char('k') => { field = field.saturating_sub(1); }
                KeyCode::Down | KeyCode::Char('j') => { if field < 1 { field += 1; } }
                KeyCode::Left | KeyCode::Char('h') => {
                    if field == 0 && source_idx > 0 { source_idx -= 1; }
                    if field == 1 && target_idx > 0 { target_idx -= 1; }
                }
                KeyCode::Right | KeyCode::Char('l') => {
                    if field == 0 && source_idx + 1 < source_inits.len() { source_idx += 1; }
                    if field == 1 && target_idx + 1 < target_inits.len() { target_idx += 1; }
                }
                KeyCode::Enter => {
                    if source_inits[source_idx] == target_inits[target_idx] {
                        continue; // Same source and target?
                    }
                    let result = serde_json::json!({
                        "source": source_inits[source_idx],
                        "target": target_inits[target_idx]
                    });
                    break Response { result: Some(result), cancelled: false, error: None };
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