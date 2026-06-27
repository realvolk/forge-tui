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
use std::io;

pub fn run(title: String, message: String) -> Result<Response> {
    let mut stdout = io::stdout();
    crossterm::terminal::enable_raw_mode()?;
    crossterm::execute!(stdout, crossterm::terminal::EnterAlternateScreen)?;
    crossterm::execute!(stdout, crossterm::cursor::Hide)?;

    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let theme = Theme::load();

    loop {
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
                .constraints([Constraint::Min(1), Constraint::Length(1)])
                .split(inner);

            let msg = Paragraph::new(message.as_str())
                .style(theme.normal_style)
                .wrap(Wrap { trim: false });
            f.render_widget(msg, chunks[0]);

            let hint = Paragraph::new("Press any key to continue")
                .style(theme.muted_style)
                .alignment(Alignment::Center);
            f.render_widget(hint, chunks[1]);
        })?;

        if let Event::Key(_) = event::read()? {
            break;
        }
    }

    crossterm::execute!(terminal.backend_mut(), crossterm::cursor::Show)?;
    crossterm::execute!(terminal.backend_mut(), crossterm::terminal::LeaveAlternateScreen)?;
    crossterm::terminal::disable_raw_mode()?;
    Ok(Response { result: None, cancelled: false, error: None })
}