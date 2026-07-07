use crate::contract::Response;
use crate::theme::Theme;
use crate::widgets::helpers;
use anyhow::Result;
use crossterm::event::{self, Event, KeyCode};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Alignment, Constraint, Layout},
    text::{Line, Span},
    widgets::{Paragraph, Wrap},
    Terminal, Frame,
};
use std::fs::File;

pub fn run(
    terminal: Option<&mut Terminal<CrosstermBackend<File>>>,
    title: String,
    message: String,
    default: Option<bool>,
) -> Result<Response> {
    run_with_background(terminal, title, message, default, None)
}

pub fn run_with_background(
    terminal: Option<&mut Terminal<CrosstermBackend<File>>>,
    title: String,
    message: String,
    default: Option<bool>,
    background: Option<&dyn Fn(&mut Frame)>,
) -> Result<Response> {
    let is_daemon = terminal.is_some();
    let theme = Theme::load();
    let default_yes = default.unwrap_or(true);
    let mut selected_yes = default_yes;

    let mut owned;
    let term: &mut Terminal<CrosstermBackend<File>> = match terminal {
        Some(t) => t,
        None => {
            owned = helpers::setup_one_shot()?;
            &mut owned
        }
    };

    let result = loop {
        term.draw(|f: &mut Frame| {
            let area = f.size();
            if let Some(ref wm) = theme.watermark_path {
                crate::watermark::render(f, area, wm);
            }
            let inner = helpers::render_overlay(f, area, &title, 50, 35, background);

            let chunks = Layout::default()
                .constraints([Constraint::Min(1), Constraint::Length(3), Constraint::Length(1)])
                .split(inner);

            if !message.is_empty() {
                f.render_widget(
                    Paragraph::new(message.as_str())
                        .style(theme.normal_style)
                        .wrap(Wrap { trim: false }),
                    chunks[0],
                );
            }

            let yes_style = if selected_yes {
                theme.selected_style
            } else {
                theme.muted_style
            };
            let no_style = if !selected_yes {
                theme.selected_style
            } else {
                theme.muted_style
            };
            f.render_widget(
                Paragraph::new(Line::from(vec![
                    Span::styled("  [ Yes ]  ", yes_style),
                    Span::styled("  [ No ]  ", no_style),
                ]))
                .alignment(Alignment::Center),
                chunks[1],
            );

            f.render_widget(
                helpers::footer("h/l:choose  Enter:confirm  y/n:quick  Esc:cancel  Ctrl+C:quit"),
                chunks[2],
            );
        })?;

        match event::read()? {
            Event::Key(key) => {
                if helpers::is_cancel(&Event::Key(key)) {
                    break Response {
                        result: None,
                        cancelled: true,
                        error: None,
                    };
                }
                match (key.code, key.modifiers) {
                    (KeyCode::Left, _) | (KeyCode::Char('h'), _) | (KeyCode::Tab, _) => {
                        selected_yes = !selected_yes;
                    }
                    (KeyCode::Right, _) | (KeyCode::Char('l'), _) => {
                        selected_yes = !selected_yes;
                    }
                    (KeyCode::Enter, _) => {
                        break Response {
                            result: Some(serde_json::Value::Bool(selected_yes)),
                            cancelled: false,
                            error: None,
                        };
                    }
                    (KeyCode::Char('y'), _) => {
                        break Response {
                            result: Some(serde_json::Value::Bool(true)),
                            cancelled: false,
                            error: None,
                        };
                    }
                    (KeyCode::Char('n'), _) => {
                        break Response {
                            result: Some(serde_json::Value::Bool(false)),
                            cancelled: false,
                            error: None,
                        };
                    }
                    _ => {}
                }
            }
            _ => {}
        }
    };

    if !is_daemon {
        helpers::teardown_one_shot()?;
    }
    Ok(result)
}