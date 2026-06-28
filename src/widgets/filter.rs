use crate::contract::Response;
use crate::layout;
use crate::theme::Theme;
use anyhow::Result;
use crossterm::event::{self, Event, KeyCode};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Layout, Margin},
    widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph, Wrap},
    Terminal, Frame,
};
use std::fs::File;
use std::io;

pub fn run(
    terminal: Option<&mut Terminal<CrosstermBackend<File>>>,
    title: String,
    message: String,
    choices: Vec<String>,
    placeholder: Option<String>,
) -> Result<Response> {
    let is_daemon = terminal.is_some();
    let theme = Theme::load();
    let mut query = String::new();
    let mut list_state = ListState::default().with_selected(Some(0));

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
        let filtered: Vec<&String> = if query.is_empty() { choices.iter().collect() } else { let q = query.to_lowercase(); choices.iter().filter(|c| c.to_lowercase().contains(&q)).collect() };
        if let Some(sel) = list_state.selected() { if sel >= filtered.len() && !filtered.is_empty() { list_state.select(Some(filtered.len() - 1)); } }
        if filtered.is_empty() { list_state.select(None); }

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
            let constraints: Vec<Constraint> = if has_msg { vec![Constraint::Length(2), Constraint::Length(3), Constraint::Min(1)] } else { vec![Constraint::Length(3), Constraint::Min(1)] };
            let chunks = Layout::default().constraints(constraints).split(inner);
            let mut offset = 0;
            if has_msg { f.render_widget(Paragraph::new(message.as_str()).style(theme.normal_style).wrap(Wrap { trim: false }), chunks[0]); offset = 1; }

            let sd = if query.is_empty() { placeholder.as_deref().unwrap_or("Type to filter...") } else { &query };
            let ss = if query.is_empty() { theme.muted_style } else { theme.accent_style };
            f.render_widget(Paragraph::new(format!("> {}", sd)).style(ss), chunks[offset]);
            f.set_cursor(chunks[offset].x + 2 + query.len() as u16, chunks[offset].y);

            let items: Vec<ListItem> = filtered.iter().map(|c| ListItem::new(c.as_str())).collect();
            f.render_stateful_widget(List::new(items).highlight_style(theme.selected_style).highlight_symbol(" >"), chunks[offset + 1], &mut list_state.clone());
        })?;

        match event::read()? {
            Event::Key(key) => match key.code {
                KeyCode::Enter => { if let Some(idx) = list_state.selected() { if let Some(choice) = filtered.get(idx) { break Response { result: Some(serde_json::Value::String(choice.to_string())), cancelled: false, error: None }; } } }
                KeyCode::Esc => break Response { result: None, cancelled: true, error: None },
                KeyCode::Up | KeyCode::Char('k') => { let i = list_state.selected().unwrap_or(0); if i > 0 { list_state.select(Some(i - 1)); } }
                KeyCode::Down | KeyCode::Char('j') => { let i = list_state.selected().unwrap_or(0); if i < filtered.len().saturating_sub(1) { list_state.select(Some(i + 1)); } }
                KeyCode::Char(c) => { query.push(c); list_state.select(Some(0)); }
                KeyCode::Backspace => { query.pop(); list_state.select(Some(0)); }
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