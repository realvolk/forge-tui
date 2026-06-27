use crate::contract::Response;
use crate::layout;
use crate::theme::Theme;
use anyhow::Result;
use crossterm::event::{self, Event, KeyCode};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Layout, Margin},
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
    Terminal, Frame,
};
use std::io;

pub fn run(title: String, message: String, placeholder: Option<String>) -> Result<Response> {
    let mut stdout = io::stdout();
    crossterm::terminal::enable_raw_mode()?;
    crossterm::execute!(stdout, crossterm::terminal::EnterAlternateScreen)?;
    crossterm::execute!(stdout, crossterm::cursor::Hide)?;

    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let theme = Theme::load();
    let mut password = String::new();

    let result = loop {
        let masked = "*".repeat(password.len());
        let display = if masked.is_empty() {
            placeholder.as_deref().unwrap_or("")
        } else {
            &masked
        };

        terminal.draw(|f: &mut Frame| {
            let area = f.size();

            if let Some(ref wm) = theme.watermark_path {
                crate::watermark::render(f, area, wm);
            }

            let box_area = layout::centered(50, 20, area);
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
                vec![Constraint::Length(2), Constraint::Length(3)]
            } else {
                vec![Constraint::Length(3)]
            };
            let chunks = Layout::default().constraints(constraints).split(inner);

            if has_message {
                let msg = Paragraph::new(message.as_str())
                    .style(theme.normal_style)
                    .wrap(Wrap { trim: false });
                f.render_widget(msg, chunks[0]);
            }

            let input_chunk = if has_message { chunks[1] } else { chunks[0] };
            let style = if password.is_empty() { theme.muted_style } else { theme.accent_style };
            f.render_widget(
                Paragraph::new(format!("> {}", display)).style(style),
                input_chunk,
            );
        })?;

        match event::read()? {
            Event::Key(key) => match key.code {
                KeyCode::Enter => {
                    break Response {
                        result: Some(serde_json::Value::String(password)),
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
                KeyCode::Char(c) => password.push(c),
                KeyCode::Backspace => { password.pop(); }
                _ => {}
            },
            _ => {}
        }
    };

    crossterm::execute!(terminal.backend_mut(), crossterm::cursor::Show)?;
    crossterm::execute!(terminal.backend_mut(), crossterm::terminal::LeaveAlternateScreen)?;
    crossterm::terminal::disable_raw_mode()?;
    Ok(result)
}