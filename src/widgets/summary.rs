use crate::contract::Response;
use crate::theme::Theme;
use crate::widgets::helpers;
use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, MouseEventKind};
use ratatui::{
    backend::CrosstermBackend,
    layout::Rect,
    widgets::{Paragraph, Scrollbar, ScrollbarOrientation, Wrap},
    Terminal, Frame,
};
use std::fs;
use std::fs::File;

pub fn run(
    terminal: Option<&mut Terminal<CrosstermBackend<File>>>,
    title: String,
    message: Option<String>,
    file: Option<String>,
) -> Result<Response> {
    let text = if let Some(ref path) = file {
        fs::read_to_string(path).unwrap_or_else(|_| format!("[Error reading {}]", path))
    } else {
        message.unwrap_or_default()
    };

    let is_daemon = terminal.is_some();
    let theme = Theme::load();
    let total_lines = text.lines().count() as u16;
    let mut scroll: u16 = 0;

    let mut owned;
    let term: &mut Terminal<CrosstermBackend<File>> = match terminal {
        Some(t) => t,
        None => {
            helpers::enable_mouse()?;
            owned = helpers::setup_one_shot()?;
            &mut owned
        }
    };

    loop {
        term.draw(|f: &mut Frame| {
            let area = f.size();
            if let Some(ref wm) = theme.watermark_path { crate::watermark::render(f, area, wm); }
            let inner = helpers::render_box(f, area, &title);

            let visible_height = inner.height.saturating_sub(2);
            let max_scroll = total_lines.saturating_sub(visible_height);
            scroll = scroll.min(max_scroll);

            f.render_widget(
                Paragraph::new(text.as_str())
                    .style(theme.normal_style)
                    .wrap(Wrap { trim: false })
                    .scroll((scroll, 0)),
                inner,
            );

            if max_scroll > 0 {
                let mut sb = ratatui::widgets::ScrollbarState::new(max_scroll as usize).position(scroll as usize);
                f.render_stateful_widget(
                    Scrollbar::default().orientation(ScrollbarOrientation::VerticalRight),
                    inner,
                    &mut sb,
                );
            }

            let hint_y = inner.y + inner.height.saturating_sub(1);
            if hint_y < area.height {
                f.render_widget(
                    helpers::footer("j/k:scroll  PgUp/PgDn  Home/End  q:quit  Ctrl+C:quit"),
                    Rect::new(inner.x, hint_y, inner.width, 1),
                );
            }
        })?;

        match event::read()? {
            Event::Key(key) => {
                if helpers::is_cancel(&Event::Key(key)) {
                    return Ok(Response { result: None, cancelled: true, error: None });
                }
                match key.code {
                    KeyCode::Up | KeyCode::Char('k') => scroll = scroll.saturating_sub(1),
                    KeyCode::Down | KeyCode::Char('j') => scroll = (scroll + 1).min(total_lines.saturating_sub(1)),
                    KeyCode::PageUp => scroll = scroll.saturating_sub(10),
                    KeyCode::PageDown => scroll = (scroll + 10).min(total_lines.saturating_sub(1)),
                    KeyCode::Home => scroll = 0,
                    KeyCode::End => scroll = total_lines.saturating_sub(1),
                    KeyCode::Enter | KeyCode::Char('q') => break,
                    _ => {}
                }
            }
            Event::Mouse(m) => match m.kind {
                MouseEventKind::ScrollDown => scroll = (scroll + 3).min(total_lines.saturating_sub(1)),
                MouseEventKind::ScrollUp => scroll = scroll.saturating_sub(3),
                _ => {}
            },
            _ => {}
        }
    }

    if !is_daemon {
        helpers::disable_mouse()?;
        helpers::teardown_one_shot()?;
    }
    Ok(Response { result: None, cancelled: false, error: None })
}