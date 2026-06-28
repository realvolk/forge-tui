use crate::contract::Response;
use crate::layout;
use crate::theme::Theme;
use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, MouseEventKind};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Layout, Margin},
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
    _height: Option<u16>,
    min: Option<usize>,
    max: Option<usize>,
    default: Option<Vec<String>>,
) -> Result<Response> {
    let is_daemon = terminal.is_some();
    let theme = Theme::load();

    let default_set: HashSet<String> = default.unwrap_or_default().into_iter().collect();
    let mut selected: HashSet<usize> = choices.iter().enumerate().filter(|(_, c)| default_set.contains(*c)).map(|(i, _)| i).collect();
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
            crossterm::execute!(io::stdout(), crossterm::event::EnableMouseCapture)?;
            crossterm::execute!(io::stdout(), crossterm::cursor::Hide)?;
            owned_terminal = Terminal::new(CrosstermBackend::new(stdout))?;
            &mut owned_terminal
        }
    };

    let result = loop {
        terminal.draw(|f: &mut Frame| {
            let area = f.size();
            if let Some(ref wm) = theme.watermark_path { crate::watermark::render(f, area, wm); }

            let box_area = layout::centered(70, 70, area);
            f.render_widget(Clear, box_area);
            let block = Block::default().borders(Borders::ALL).border_style(theme.border_style)
                .title(title.as_str()).title_style(theme.title_style);
            f.render_widget(block, box_area);

            let inner = box_area.inner(&Margin::new(2, 1));
            let has_msg = !message.is_empty();
            let constraints: Vec<Constraint> = if has_msg { vec![Constraint::Length(2), Constraint::Min(1), Constraint::Length(1)] } else { vec![Constraint::Min(1), Constraint::Length(1)] };
            let chunks = Layout::default().constraints(constraints).split(inner);
            if has_msg { f.render_widget(Paragraph::new(message.as_str()).style(theme.normal_style).wrap(Wrap { trim: false }), chunks[0]); }

            let li = if has_msg { 1 } else { 0 };
            let si = if has_msg { 2 } else { 1 };

            let items: Vec<ListItem> = choices.iter().enumerate().map(|(i, c)| {
                let mark = if selected.contains(&i) { "[x]" } else { "[ ]" };
                let style = if selected.contains(&i) { theme.accent_style } else { theme.normal_style };
                ListItem::new(format!(" {} {}", mark, c)).style(style)
            }).collect();

            f.render_stateful_widget(List::new(items).highlight_style(theme.selected_style).highlight_symbol(" >"), chunks[li], &mut list_state.clone());
            f.render_widget(Paragraph::new(format!(" Selected: {}/{}   Space=toggle  Enter=confirm  Esc=cancel", selected.len(), choices.len())).style(theme.muted_style), chunks[si]);
        })?;

        match event::read()? {
            Event::Key(key) => match key.code {
                KeyCode::Up | KeyCode::Char('k') => { let i = list_state.selected().unwrap_or(0); if i > 0 { list_state.select(Some(i - 1)); } }
                KeyCode::Down | KeyCode::Char('j') => { let i = list_state.selected().unwrap_or(0); if i < choices.len().saturating_sub(1) { list_state.select(Some(i + 1)); } }
                KeyCode::Char(' ') => {
                    let i = list_state.selected().unwrap_or(0);
                    if selected.contains(&i) { if selected.len() > min_items { selected.remove(&i); } }
                    else if selected.len() < max_items { selected.insert(i); }
                }
                KeyCode::Enter => {
                    if selected.len() >= min_items {
                        let rc: Vec<String> = selected.iter().map(|&i| choices[i].clone()).collect();
                        break Response { result: Some(serde_json::Value::Array(rc.into_iter().map(serde_json::Value::String).collect())), cancelled: false, error: None };
                    }
                }
                KeyCode::Esc => break Response { result: None, cancelled: true, error: None },
                _ => {}
            },
            Event::Mouse(mouse) => match mouse.kind {
                MouseEventKind::ScrollDown => { let i = list_state.selected().unwrap_or(0); if i < choices.len().saturating_sub(1) { list_state.select(Some(i + 1)); } }
                MouseEventKind::ScrollUp => { let i = list_state.selected().unwrap_or(0); if i > 0 { list_state.select(Some(i - 1)); } }
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