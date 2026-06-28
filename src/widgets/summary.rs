use crate::contract::Response;
use crate::layout;
use crate::theme::Theme;
use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, MouseEventKind};
use ratatui::{
    backend::CrosstermBackend,
    layout::Margin,
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Scrollbar, ScrollbarOrientation, Wrap},
    Terminal, Frame,
};
use std::fs;
use std::fs::File;
use std::io;

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

    loop {
        terminal.draw(|f: &mut Frame| {
            let area = f.size();
            if let Some(ref wm) = theme.watermark_path { crate::watermark::render(f, area, wm); }

            let box_area = layout::centered(80, 80, area);
            f.render_widget(Clear, box_area);
            let block = Block::default().borders(Borders::ALL).border_style(theme.border_style)
                .title(title.as_str()).title_style(theme.title_style);
            f.render_widget(block, box_area);

            let inner = box_area.inner(&Margin::new(2, 1));
            let visible_height = inner.height;
            let max_scroll = total_lines.saturating_sub(visible_height);

            f.render_widget(Paragraph::new(text.as_str()).style(theme.normal_style).wrap(Wrap { trim: false }).scroll((scroll, 0)), inner);

            if max_scroll > 0 {
                let mut sb = ratatui::widgets::ScrollbarState::new(max_scroll as usize).position(scroll as usize);
                f.render_stateful_widget(Scrollbar::default().orientation(ScrollbarOrientation::VerticalRight), inner, &mut sb);
            }

            let hy = box_area.y + box_area.height.saturating_sub(1);
            let hx = box_area.x + 2;
            if hy < area.height && hx < area.width {
                f.render_widget(Paragraph::new(Line::from(Span::styled(" j/k:scroll  PgUp/PgDn  Home/End  q:quit ", theme.muted_style))), ratatui::layout::Rect::new(hx, hy, box_area.width.saturating_sub(4), 1));
            }
        })?;

        match event::read()? {
            Event::Key(key) => match key.code {
                KeyCode::Up | KeyCode::Char('k') => scroll = scroll.saturating_sub(1),
                KeyCode::Down | KeyCode::Char('j') => scroll = (scroll + 1).min(total_lines.saturating_sub(1)),
                KeyCode::PageUp => scroll = scroll.saturating_sub(10),
                KeyCode::PageDown => scroll = (scroll + 10).min(total_lines.saturating_sub(1)),
                KeyCode::Home => scroll = 0,
                KeyCode::End => scroll = total_lines.saturating_sub(1),
                KeyCode::Enter | KeyCode::Esc | KeyCode::Char('q') => break,
                _ => {}
            },
            Event::Mouse(mouse) => match mouse.kind {
                MouseEventKind::ScrollDown => scroll = (scroll + 3).min(total_lines.saturating_sub(1)),
                MouseEventKind::ScrollUp => scroll = scroll.saturating_sub(3),
                _ => {}
            },
            _ => {}
        }
    }

    if !is_daemon {
        crossterm::execute!(io::stdout(), crossterm::cursor::Show)?;
        crossterm::execute!(io::stdout(), crossterm::event::DisableMouseCapture)?;
        crossterm::execute!(io::stdout(), crossterm::terminal::LeaveAlternateScreen)?;
        crossterm::terminal::disable_raw_mode()?;
    }
    Ok(Response { result: None, cancelled: false, error: None })
}