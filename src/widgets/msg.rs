use crate::contract::Response;
use crate::theme::Theme;
use crate::widgets::helpers;
use anyhow::Result;
use crossterm::event::{self, Event};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Layout},
    widgets::{Paragraph, Wrap},
    Terminal, Frame,
};
use std::fs::File;

pub fn run(
    terminal: Option<&mut Terminal<CrosstermBackend<File>>>,
    title: String,
    message: String,
) -> Result<Response> {
    let is_daemon = terminal.is_some();
    let theme = Theme::load();

    let mut owned;
    let term: &mut Terminal<CrosstermBackend<File>> = match terminal {
        Some(t) => t,
        None => {
            owned = helpers::setup_one_shot()?;
            &mut owned
        }
    };

    loop {
        term.draw(|f: &mut Frame| {
            let area = f.size();
            if let Some(ref wm) = theme.watermark_path { crate::watermark::render(f, area, wm); }
            let inner = helpers::render_box(f, area, &title);

            let chunks = Layout::default()
                .constraints([Constraint::Min(1), Constraint::Length(1)])
                .split(inner);

            f.render_widget(
                Paragraph::new(message.as_str()).style(theme.normal_style).wrap(Wrap { trim: false }),
                chunks[0],
            );
            f.render_widget(helpers::footer("Any key:continue  Ctrl+C:quit"), chunks[1]);
        })?;

        match event::read()? {
            Event::Key(key) => {
                if helpers::is_cancel(&Event::Key(key)) {
                    return Ok(Response { result: None, cancelled: true, error: None });
                }
                break;
            }
            _ => {}
        }
    }

    if !is_daemon { helpers::teardown_one_shot()?; }
    Ok(Response { result: None, cancelled: false, error: None })
}