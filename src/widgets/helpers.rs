use anyhow::Result;
use crossterm::event::{Event, KeyCode, KeyModifiers};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Alignment, Rect},
    text::{Line, Span},
    widgets::{Clear, Paragraph},
    Terminal, Frame,
};
use std::fs::File;
use std::io;

pub fn setup_one_shot() -> Result<Terminal<CrosstermBackend<File>>> {
    let stdout = crate::tty::open()?;
    crossterm::terminal::enable_raw_mode()?;
    crossterm::execute!(io::stdout(), crossterm::terminal::EnterAlternateScreen)?;
    crossterm::execute!(io::stdout(), crossterm::cursor::Hide)?;
    Ok(Terminal::new(CrosstermBackend::new(stdout))?)
}

pub fn teardown_one_shot() -> Result<()> {
    crossterm::execute!(io::stdout(), crossterm::cursor::Show)?;
    crossterm::execute!(io::stdout(), crossterm::terminal::LeaveAlternateScreen)?;
    crossterm::terminal::disable_raw_mode()?;
    Ok(())
}

pub fn is_cancel(key: &Event) -> bool {
    match key {
        Event::Key(k) => {
            (k.code == KeyCode::Char('c') && k.modifiers == KeyModifiers::CONTROL)
                || k.code == KeyCode::Esc
        }
        _ => false,
    }
}

pub fn is_confirm(key: &Event) -> bool {
    matches!(key, Event::Key(k) if k.code == KeyCode::Enter)
}

pub fn footer(text: &str) -> Paragraph {
    Paragraph::new(Line::from(Span::styled(
        format!(" {} ", text),
        crate::theme::Theme::load().muted_style,
    )))
    .alignment(Alignment::Center)
}

pub fn render_box(f: &mut Frame, area: Rect, title: &str) -> Rect {
    use crate::layout;
    use crate::theme::Theme;
    use ratatui::widgets::{Block, Borders};

    let theme = Theme::load();
    let box_area = layout::centered(80, 80, area);
    f.render_widget(Clear, box_area);
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(theme.border_style)
        .title(title)
        .title_style(theme.title_style);
    f.render_widget(block, box_area);
    box_area.inner(&ratatui::layout::Margin::new(2, 1))
}

pub fn enable_mouse() -> Result<()> {
    crossterm::execute!(io::stdout(), crossterm::event::EnableMouseCapture)?;
    Ok(())
}

pub fn disable_mouse() -> Result<()> {
    crossterm::execute!(io::stdout(), crossterm::event::DisableMouseCapture)?;
    Ok(())
}