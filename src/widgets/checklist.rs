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
use std::io;

pub fn run(
    title: String,
    message: String,
    choices: Vec<String>,
    _height: Option<u16>,
    min: Option<usize>,
    max: Option<usize>,
    default: Option<Vec<String>>,
) -> Result<Response> {
    let mut stdout = io::stdout();
    crossterm::terminal::enable_raw_mode()?;
    crossterm::execute!(stdout, crossterm::terminal::EnterAlternateScreen)?;
    crossterm::execute!(stdout, crossterm::event::EnableMouseCapture)?;
    crossterm::execute!(stdout, crossterm::cursor::Hide)?;

    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let theme = Theme::load();

    let default_set: HashSet<String> = default.unwrap_or_default().into_iter().collect();
    let mut selected: HashSet<usize> = choices
        .iter()
        .enumerate()
        .filter(|(_, c)| default_set.contains(*c))
        .map(|(i, _)| i)
        .collect();

    let mut list_state = ListState::default().with_selected(Some(0));
    let min_items = min.unwrap_or(0);
    let max_items = max.unwrap_or(usize::MAX);

    let result = loop {
        terminal.draw(|f: &mut Frame| {
            let area = f.size();

            if let Some(ref wm) = theme.watermark_path {
                crate::watermark::render(f, area, wm);
            }

            let box_area = layout::centered(70, 70, area);
            f.render_widget(Clear, box_area);

            let block = Block::default()
                .borders(Borders::ALL)
                .border_style(theme.border_style)
                .title(title.as_str())
                .title_style(theme.title_style);
            f.render_widget(block, box_area);

            let inner = box_area.inner(&Margin::new(2, 1));

            let has_message = !message.is_empty();
            let constraints: Vec<Constraint> = if has_message {
                vec![Constraint::Length(2), Constraint::Min(1), Constraint::Length(1)]
            } else {
                vec![Constraint::Min(1), Constraint::Length(1)]
            };
            let chunks = Layout::default().constraints(constraints).split(inner);

            if has_message {
                let msg = Paragraph::new(message.as_str())
                    .style(theme.normal_style)
                    .wrap(Wrap { trim: false });
                f.render_widget(msg, chunks[0]);
            }

            let list_idx = if has_message { 1 } else { 0 };
            let status_idx = if has_message { 2 } else { 1 };

            let items: Vec<ListItem> = choices
                .iter()
                .enumerate()
                .map(|(i, c)| {
                    let mark = if selected.contains(&i) { "[x]" } else { "[ ]" };
                    let style = if selected.contains(&i) { theme.accent_style } else { theme.normal_style };
                    ListItem::new(format!(" {} {}", mark, c)).style(style)
                })
                .collect();

            let list = List::new(items)
                .highlight_style(theme.selected_style)
                .highlight_symbol(" >");
            f.render_stateful_widget(list, chunks[list_idx], &mut list_state.clone());

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
                KeyCode::Up | KeyCode::Char('k') => {
                    let i = list_state.selected().unwrap_or(0);
                    if i > 0 { list_state.select(Some(i - 1)); }
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    let i = list_state.selected().unwrap_or(0);
                    if i < choices.len().saturating_sub(1) {
                        list_state.select(Some(i + 1));
                    }
                }
                KeyCode::Char(' ') => {
                    let i = list_state.selected().unwrap_or(0);
                    if selected.contains(&i) {
                        if selected.len() > min_items { selected.remove(&i); }
                    } else if selected.len() < max_items {
                        selected.insert(i);
                    }
                }
                KeyCode::Enter => {
                    if selected.len() >= min_items {
                        let result_choices: Vec<String> = selected.iter().map(|&i| choices[i].clone()).collect();
                        break Response {
                            result: Some(serde_json::Value::Array(
                                result_choices.into_iter().map(serde_json::Value::String).collect(),
                            )),
                            cancelled: false,
                            error: None,
                        };
                    }
                }
                KeyCode::Esc => {
                    break Response { result: None, cancelled: true, error: None };
                }
                _ => {}
            },
            Event::Mouse(mouse) => match mouse.kind {
                MouseEventKind::ScrollDown => {
                    let i = list_state.selected().unwrap_or(0);
                    if i < choices.len().saturating_sub(1) {
                        list_state.select(Some(i + 1));
                    }
                }
                MouseEventKind::ScrollUp => {
                    let i = list_state.selected().unwrap_or(0);
                    if i > 0 { list_state.select(Some(i - 1)); }
                }
                _ => {}
            },
            _ => {}
        }
    };

    crossterm::execute!(terminal.backend_mut(), crossterm::cursor::Show)?;
    crossterm::execute!(terminal.backend_mut(), crossterm::event::DisableMouseCapture)?;
    crossterm::execute!(terminal.backend_mut(), crossterm::terminal::LeaveAlternateScreen)?;
    crossterm::terminal::disable_raw_mode()?;
    Ok(result)
}