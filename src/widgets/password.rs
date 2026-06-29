use crate::contract::Response;
use crate::layout;
use crate::theme::Theme;
use anyhow::Result;
use crossterm::event::{self, Event, KeyCode};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Layout, Margin},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Gauge, Paragraph, Wrap},
    Terminal, Frame,
};
use std::fs::File;
use std::io;

pub fn run(
    terminal: Option<&mut Terminal<CrosstermBackend<File>>>,
    title: String,
    message: String,
    placeholder: Option<String>,
) -> Result<Response> {
    let is_daemon = terminal.is_some();
    let theme = Theme::load();
    let mut password = String::new();

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

    let result = loop {
        let masked = "*".repeat(password.len());
        let display = if masked.is_empty() { placeholder.as_deref().unwrap_or("") } else { &masked };
        let (score, label, bar_color) = password_strength(&password);

        terminal.draw(|f: &mut Frame| {
            let area = f.size();
            if let Some(ref wm) = theme.watermark_path { crate::watermark::render(f, area, wm); }

            let box_area = layout::centered(50, 25, area);
            f.render_widget(Clear, box_area);

            let block = Block::default().borders(Borders::ALL).border_style(theme.border_style)
                .title(title.as_str()).title_style(theme.title_style);
            f.render_widget(block, box_area);

            let inner = box_area.inner(&Margin::new(2, 1));
            let has_msg = !message.is_empty();
            let constraints: Vec<Constraint> = if has_msg {
                vec![Constraint::Length(2), Constraint::Length(3), Constraint::Length(3)]
            } else {
                vec![Constraint::Length(3), Constraint::Length(3)]
            };
            let chunks = Layout::default().constraints(constraints).split(inner);

            if has_msg {
                f.render_widget(Paragraph::new(message.as_str()).style(theme.normal_style).wrap(Wrap { trim: false }), chunks[0]);
            }
            let ic = if has_msg { chunks[1] } else { chunks[0] };
            let sc = if has_msg { chunks[2] } else { chunks[1] };

            let style = if password.is_empty() { theme.muted_style } else { theme.accent_style };
            f.render_widget(Paragraph::new(format!("> {}", display)).style(style), ic);

            // Strength meter
            if !password.is_empty() {
                let gauge = Gauge::default()
                    .gauge_style(Style::default().fg(bar_color).add_modifier(Modifier::BOLD))
                    .percent(score as u16)
                    .label(label);
                f.render_widget(gauge, sc);
            } else {
                f.render_widget(
                    Paragraph::new(Line::from(Span::styled(" Strength: ", theme.muted_style))),
                    sc,
                );
            }
        })?;

        match event::read()? {
            Event::Key(key) => match key.code {
                KeyCode::Enter => break Response { result: Some(serde_json::Value::String(password)), cancelled: false, error: None },
                KeyCode::Esc => break Response { result: None, cancelled: true, error: None },
                KeyCode::Char(c) => password.push(c),
                KeyCode::Backspace => { password.pop(); }
                _ => {}
            },
            _ => {}
        }
    };

    if !is_daemon {
        crossterm::execute!(io::stdout(), crossterm::cursor::Show)?;
        crossterm::execute!(io::stdout(), crossterm::terminal::LeaveAlternateScreen)?;
        crossterm::terminal::disable_raw_mode()?;
    }
    Ok(result)
}

fn password_strength(password: &str) -> (u8, &'static str, Color) {
    let len = password.len();
    if len == 0 {
        return (0, "Enter password", Color::Gray);
    }

    let has_upper = password.chars().any(|c| c.is_uppercase());
    let has_lower = password.chars().any(|c| c.is_lowercase());
    let has_digit = password.chars().any(|c| c.is_ascii_digit());
    let has_special = password.chars().any(|c| !c.is_alphanumeric());

    let mut score: u8 = 0;

    // Length: up to 40 points
    score += (len.min(16) * 2) as u8;

    // Character variety
    if has_upper { score += 15; }
    if has_lower { score += 15; }
    if has_digit { score += 15; }
    if has_special { score += 15; }

    // Mixed case bonus
    if has_upper && has_lower { score += 10; }

    // Length bonuses
    if len >= 12 { score += 10; }
    if len >= 16 { score += 10; }

    // Cap at 100
    score = score.min(100);

    let (label, color) = match score {
        0..=25 => ("Weak", Color::Red),
        26..=50 => ("Fair", Color::Yellow),
        51..=75 => ("Good", Color::Blue),
        _ => ("Strong", Color::Green),
    };

    (score, label, color)
}