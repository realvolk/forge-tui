use crate::contract::Response;
use crate::layout;
use crate::theme::Theme;
use crate::widgets;
use anyhow::Result;
use crossterm::event::{self, Event, KeyCode};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Alignment, Constraint, Layout, Margin},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph},
    Terminal, Frame,
};
use serde_json::Value;
use std::collections::HashMap;
use std::fs::File;
use std::io;

pub fn run(
    terminal: Option<&mut Terminal<CrosstermBackend<File>>>,
    title: String,
    current_de: String,
) -> Result<Response> {
    let is_daemon = terminal.is_some();
    let theme = Theme::load();

    let des = vec![
        "kde", "sonicde", "xfce", "lxqt", "lxde", "hyprland", "sway", "niri",
        "i3wm", "dwm", "vxwm", "icewm", "mango", "cinnamon", "budgie", "moksha", "cosmic", "none",
    ];

    let dm_choices = vec!["current", "sddm", "lightdm", "soniclogin", "none"];
    let x_choices = vec!["current", "xlibre", "xorg", "wayland"];
    let audio_choices = vec!["current", "pipewire", "pulseaudio", "none"];
    let net_choices = vec!["current", "networkmanager", "dhcpcd+iwd", "connman", "none"];

    let mut source_idx = des.iter().position(|d| d == &current_de).unwrap_or(0);
    let mut target_idx = 0;
    let mut dm_idx: usize = 0;
    let mut x_idx: usize = 0;
    let mut audio_idx: usize = 0;
    let mut net_idx: usize = 0;
    let mut field: usize = 0; // 0=source, 1=target, 2=dm, 3=x, 4=audio, 5=net

    let mut owned;
    let term: &mut Terminal<CrosstermBackend<File>> = match terminal {
        Some(t) => t,
        None => {
            let stdout = crate::tty::open()?;
            crossterm::terminal::enable_raw_mode()?;
            crossterm::execute!(io::stdout(), crossterm::terminal::EnterAlternateScreen)?;
            crossterm::execute!(io::stdout(), crossterm::cursor::Hide)?;
            owned = Terminal::new(CrosstermBackend::new(stdout))?;
            &mut owned
        }
    };

    let result = loop {
        term.draw(|f: &mut Frame| {
            let area = f.size();
            if let Some(ref wm) = theme.watermark_path {
                crate::watermark::render(f, area, wm);
            }

            let box_area = layout::centered(55, 45, area);
            f.render_widget(Clear, box_area);
            let block = Block::default()
                .borders(Borders::ALL)
                .border_style(theme.border_style)
                .title(title.as_str())
                .title_style(theme.title_style);
            f.render_widget(block, box_area);

            let inner = box_area.inner(&Margin::new(2, 1));
            let fields = vec![
                ("Source DE", des[source_idx]),
                ("Target DE", des[target_idx]),
                ("Display Manager", dm_choices[dm_idx]),
                ("Display Stack", x_choices[x_idx]),
                ("Audio", audio_choices[audio_idx]),
                ("Network", net_choices[net_idx]),
            ];

            let lines: Vec<Line> = fields.iter().enumerate().map(|(i, (label, val))| {
                let style = if i == field { theme.selected_style } else { theme.normal_style };
                Line::from(vec![
                    Span::styled(format!(" {}: ", label), style),
                    Span::styled(val.to_string(), theme.accent_style),
                ])
            }).collect();

            f.render_widget(Paragraph::new(lines), inner);
        })?;

        match event::read()? {
            Event::Key(key) => match key.code {
                KeyCode::Esc => break Response { result: None, cancelled: true, error: None },
                KeyCode::Up | KeyCode::Char('k') => { field = field.saturating_sub(1); }
                KeyCode::Down | KeyCode::Char('j') => { if field < 5 { field += 1; } }
                KeyCode::Left | KeyCode::Char('h') => {
                    match field {
                        0 if source_idx > 0 => source_idx -= 1,
                        1 if target_idx > 0 => target_idx -= 1,
                        2 if dm_idx > 0 => dm_idx -= 1,
                        3 if x_idx > 0 => x_idx -= 1,
                        4 if audio_idx > 0 => audio_idx -= 1,
                        5 if net_idx > 0 => net_idx -= 1,
                        _ => {}
                    }
                }
                KeyCode::Right | KeyCode::Char('l') => {
                    match field {
                        0 if source_idx + 1 < des.len() => source_idx += 1,
                        1 if target_idx + 1 < des.len() => target_idx += 1,
                        2 if dm_idx + 1 < dm_choices.len() => dm_idx += 1,
                        3 if x_idx + 1 < x_choices.len() => x_idx += 1,
                        4 if audio_idx + 1 < audio_choices.len() => audio_idx += 1,
                        5 if net_idx + 1 < net_choices.len() => net_idx += 1,
                        _ => {}
                    }
                }
                KeyCode::Enter => {
                    if des[source_idx] == des[target_idx] {
                        continue;
                    }
                    let result = serde_json::json!({
                        "source": des[source_idx],
                        "target": des[target_idx],
                        "dm": dm_choices[dm_idx],
                        "x_stack": x_choices[x_idx],
                        "audio": audio_choices[audio_idx],
                        "network": net_choices[net_idx],
                    });
                    break Response { result: Some(result), cancelled: false, error: None };
                }
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