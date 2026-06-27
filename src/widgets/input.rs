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

pub fn run(
    title: String,
    message: String,
    default: Option<String>,
    placeholder: Option<String>,
    validation: Option<String>,
) -> Result<Response> {
    crossterm::terminal::enable_raw_mode()?;
    crossterm::execute!(io::stdout(), crossterm::terminal::EnterAlternateScreen)?;
    crossterm::execute!(io::stdout(), crossterm::cursor::Hide)?;

    let backend = CrosstermBackend::new(io::stdout());
    let mut terminal = Terminal::new(backend)?;

    let theme = Theme::load();
    let mut input = default.unwrap_or_default();
    let mut cursor = input.len();

    let result = loop {
        let display_text = if input.is_empty() {
            placeholder.as_deref().unwrap_or("")
        } else {
            &input
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

            let style = if input.is_empty() { theme.muted_style } else { theme.accent_style };
            f.render_widget(
                Paragraph::new(format!("> {}", display_text)).style(style),
                input_chunk,
            );
            f.set_cursor(input_chunk.x + 2 + cursor as u16, input_chunk.y);
        })?;

        match event::read()? {
            Event::Key(key) => match key.code {
                KeyCode::Enter => {
                    if let Some(ref pattern) = validation {
                        if let Ok(re) = regex_lite::Regex::new(pattern) {
                            if !re.is_match(&input) {
                                continue;
                            }
                        }
                    }
                    break Response {
                        result: Some(serde_json::Value::String(input)),
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
                KeyCode::Char(c) => {
                    input.insert(cursor, c);
                    cursor += 1;
                }
                KeyCode::Backspace => {
                    if cursor > 0 {
                        input.remove(cursor - 1);
                        cursor -= 1;
                    }
                }
                KeyCode::Delete => {
                    if cursor < input.len() {
                        input.remove(cursor);
                    }
                }
                KeyCode::Left => {
                    if cursor > 0 { cursor -= 1; }
                }
                KeyCode::Right => {
                    if cursor < input.len() { cursor += 1; }
                }
                KeyCode::Home => cursor = 0,
                KeyCode::End => cursor = input.len(),
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