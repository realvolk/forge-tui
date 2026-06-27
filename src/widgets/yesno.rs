use crate::contract::Response;
use crate::layout;
use crate::theme::Theme;
use anyhow::Result;
use crossterm::event::{self, Event, KeyCode};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Alignment, Constraint, Layout, Margin},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
    Terminal, Frame,
};
use std::io;

pub fn run(title: String, message: String, default: Option<bool>) -> Result<Response> {
    crossterm::terminal::enable_raw_mode()?;
    crossterm::execute!(io::stdout(), crossterm::terminal::EnterAlternateScreen)?;
    crossterm::execute!(io::stdout(), crossterm::cursor::Hide)?;

    let backend = CrosstermBackend::new(io::stdout());
    let mut terminal = Terminal::new(backend)?;

    let theme = Theme::load();
    let default_yes = default.unwrap_or(true);
    let mut selected_yes = default_yes;

    let result = loop {
        terminal.draw(|f: &mut Frame| {
            let area = f.size();

            if let Some(ref wm) = theme.watermark_path {
                crate::watermark::render(f, area, wm);
            }

            let box_area = layout::centered(50, 30, area);
            f.render_widget(Clear, box_area);

            let block = Block::default()
                .borders(Borders::ALL)
                .border_style(theme.border_style)
                .title(title.as_str())
                .title_style(theme.title_style);
            f.render_widget(block, box_area);

            let inner = box_area.inner(&Margin::new(2, 1));
            let chunks = Layout::default()
                .constraints([Constraint::Min(1), Constraint::Length(3)])
                .split(inner);

            if !message.is_empty() {
                let msg = Paragraph::new(message.as_str())
                    .style(theme.normal_style)
                    .wrap(Wrap { trim: false });
                f.render_widget(msg, chunks[0]);
            }

            let yes_style = if selected_yes { theme.selected_style } else { theme.muted_style };
            let no_style = if !selected_yes { theme.selected_style } else { theme.muted_style };

            let buttons = Paragraph::new(Line::from(vec![
                Span::styled("  [ Yes ]  ", yes_style),
                Span::styled("  [ No ]  ", no_style),
            ]))
            .alignment(Alignment::Center);
            f.render_widget(buttons, chunks[1]);
        })?;

        match event::read()? {
            Event::Key(key) => match key.code {
                KeyCode::Left | KeyCode::Char('h') | KeyCode::Tab => selected_yes = !selected_yes,
                KeyCode::Right | KeyCode::Char('l') => selected_yes = !selected_yes,
                KeyCode::Enter => {
                    break Response {
                        result: Some(serde_json::Value::Bool(selected_yes)),
                        cancelled: false,
                        error: None,
                    };
                }
                KeyCode::Char('y') => {
                    break Response {
                        result: Some(serde_json::Value::Bool(true)),
                        cancelled: false,
                        error: None,
                    };
                }
                KeyCode::Char('n') => {
                    break Response {
                        result: Some(serde_json::Value::Bool(false)),
                        cancelled: false,
                        error: None,
                    };
                }
                KeyCode::Esc => {
                    break Response {
                        result: None,
                        cancelled: true,
                        error: None,
                    };
                }
                _ => {}
            },
            _ => {}
        }
    };

    crossterm::execute!(io::stdout(), crossterm::cursor::Show)?;
    crossterm::execute!(io::stdout(), crossterm::terminal::LeaveAlternateScreen)?;
    crossterm::terminal::disable_raw_mode()?;
    Ok(result)
}