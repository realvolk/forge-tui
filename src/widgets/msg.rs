use crate::contract::Response;
use crate::layout;
use crate::theme::Theme;
use anyhow::Result;
use crossterm::event::{self, Event};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Alignment, Constraint, Layout, Margin},
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
    Terminal, Frame,
};
use std::fs::File;
use std::io;

pub fn run(
    terminal: Option<&mut Terminal<CrosstermBackend<File>>>,
    title: String,
    message: String,
) -> Result<Response> {
    let is_daemon = terminal.is_some();
    let theme = Theme::load();

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

    loop {
        terminal.draw(|f: &mut Frame| {
            let area = f.size();
            if let Some(ref wm) = theme.watermark_path { crate::watermark::render(f, area, wm); }

            let box_area = layout::centered(50, 30, area);
            f.render_widget(Clear, box_area);
            let block = Block::default().borders(Borders::ALL).border_style(theme.border_style)
                .title(title.as_str()).title_style(theme.title_style);
            f.render_widget(block, box_area);

            let inner = box_area.inner(&Margin::new(2, 1));
            let chunks = Layout::default().constraints([Constraint::Min(1), Constraint::Length(1)]).split(inner);
            f.render_widget(Paragraph::new(message.as_str()).style(theme.normal_style).wrap(Wrap { trim: false }), chunks[0]);
            f.render_widget(Paragraph::new("Press any key to continue").style(theme.muted_style).alignment(Alignment::Center), chunks[1]);
        })?;

        if let Event::Key(_) = event::read()? { break; }
    }

    if !is_daemon {
        crossterm::execute!(io::stdout(), crossterm::cursor::Show)?;
        crossterm::execute!(io::stdout(), crossterm::terminal::LeaveAlternateScreen)?;
        crossterm::terminal::disable_raw_mode()?;
    }
    Ok(Response { result: None, cancelled: false, error: None })
}