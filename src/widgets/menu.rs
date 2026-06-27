use crate::contract::Response;
use crate::layout;
use crate::theme::Theme;
use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, MouseEventKind};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Layout, Margin},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph, Wrap},
    Terminal, Frame,
};
use serde_json::Value;
use std::collections::HashMap;
use std::io;

struct Choice {
    label: String,
    meta: Option<String>,
    stability: Option<String>,
    warning: Option<String>,
}

impl Choice {
    fn from_value(v: &Value) -> Self {
        match v {
            Value::String(s) => Choice {
                label: s.clone(),
                meta: None,
                stability: None,
                warning: None,
            },
            Value::Object(obj) => Choice {
                label: obj.get("label").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                meta: obj.get("meta").and_then(|v| v.as_str()).map(String::from),
                stability: obj.get("stability").and_then(|v| v.as_str()).map(String::from),
                warning: obj.get("warning").and_then(|v| v.as_str()).map(String::from),
            },
            _ => Choice {
                label: String::new(),
                meta: None,
                stability: None,
                warning: None,
            },
        }
    }
}

fn parse_color(name: &str) -> Color {
    match name.to_lowercase().as_str() {
        "red" => Color::Red,
        "green" => Color::Green,
        "yellow" => Color::Yellow,
        "blue" => Color::Blue,
        "magenta" => Color::Magenta,
        "cyan" => Color::Cyan,
        "white" => Color::White,
        "gray" | "grey" => Color::Gray,
        "darkgray" => Color::DarkGray,
        "lightred" => Color::LightRed,
        "lightgreen" => Color::LightGreen,
        "lightyellow" => Color::LightYellow,
        "lightblue" => Color::LightBlue,
        "lightmagenta" => Color::LightMagenta,
        "lightcyan" => Color::LightCyan,
        "black" => Color::Black,
        _ => Color::Gray,
    }
}

pub fn run(
    title: String,
    message: String,
    choices_json: Value,
    default: Option<String>,
    stability_colors: Option<HashMap<String, String>>,
) -> Result<Response> {
    let choices: Vec<Choice> = match &choices_json {
        Value::Array(arr) => arr.iter().map(Choice::from_value).collect(),
        _ => vec![],
    };

    let mut stdout = io::stdout();
    crossterm::terminal::enable_raw_mode()?;
    crossterm::execute!(stdout, crossterm::terminal::EnterAlternateScreen)?;
    crossterm::execute!(stdout, crossterm::event::EnableMouseCapture)?;
    crossterm::execute!(stdout, crossterm::cursor::Hide)?;

    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let theme = Theme::load();

    let default_idx = default
        .as_ref()
        .and_then(|d| choices.iter().position(|c| c.label == *d))
        .unwrap_or(0);

    let mut state = ListState::default().with_selected(Some(default_idx));
    let stability_map = stability_colors.unwrap_or_default();

    let has_meta = choices.iter().any(|c| c.meta.is_some());
    let has_stability = choices.iter().any(|c| c.stability.is_some());
    let has_warnings = choices.iter().any(|c| c.warning.is_some());
    let has_info = has_warnings || has_meta;

    let result = loop {
        terminal.draw(|f: &mut Frame| {
            let area = f.size();

            if let Some(ref wm) = theme.watermark_path {
                crate::watermark::render(f, area, wm);
            }

            let box_area = layout::centered(80, 80, area);
            f.render_widget(Clear, box_area);

            let block = Block::default()
                .borders(Borders::ALL)
                .border_style(theme.border_style)
                .title(title.as_str())
                .title_style(theme.title_style);
            f.render_widget(block, box_area);

            let inner = box_area.inner(&Margin::new(2, 1));

            let has_message = !message.is_empty();
            let mut constraints: Vec<Constraint> = Vec::new();
            if has_message {
                constraints.push(Constraint::Length(2));
            }
            constraints.push(Constraint::Min(1));
            if has_info {
                constraints.push(Constraint::Length(3));
            }
            let chunks = Layout::default().constraints(constraints).split(inner);

            let mut offset: usize = 0;
            if has_message {
                let msg = Paragraph::new(message.as_str())
                    .style(theme.normal_style)
                    .wrap(Wrap { trim: false });
                f.render_widget(msg, chunks[0]);
                offset = 1;
            }

            let items: Vec<ListItem> = choices
                .iter()
                .enumerate()
                .map(|(i, choice)| {
                    let is_selected = state.selected() == Some(i);
                    let mut spans: Vec<Span> = Vec::new();

                    if is_selected {
                        spans.push(Span::styled("> ", theme.accent_style));
                    } else {
                        spans.push(Span::raw("  "));
                    }

                    let base_style = if is_selected {
                        theme.selected_style
                    } else {
                        theme.normal_style
                    };
                    spans.push(Span::styled(format!("{}  ", choice.label), base_style));

                    if has_meta {
                        let meta_text = choice.meta.as_deref().unwrap_or("");
                        spans.push(Span::styled(
                            format!("{}  ", meta_text),
                            if is_selected {
                                theme.selected_style
                            } else {
                                theme.muted_style
                            },
                        ));
                    }

                    if has_stability {
                        if let Some(ref stability) = choice.stability {
                            let color = stability_map
                                .get(stability)
                                .map(|s| parse_color(s))
                                .unwrap_or(Color::Gray);
                            spans.push(Span::styled("* ", Style::default().fg(color)));
                            spans.push(Span::styled(
                                stability.as_str(),
                                Style::default().fg(color),
                            ));
                        }
                    }

                    ListItem::new(Line::from(spans))
                })
                .collect();

            let list = List::new(items);
            f.render_stateful_widget(list, chunks[offset], &mut state.clone());

            if has_info {
                if let Some(selected_idx) = state.selected() {
                    if let Some(choice) = choices.get(selected_idx) {
                        let mut info_spans: Vec<Span> = Vec::new();

                        if let Some(ref warning) = choice.warning {
                            info_spans.push(Span::styled(
                                "!  ",
                                Style::default()
                                    .fg(Color::Yellow)
                                    .add_modifier(Modifier::BOLD),
                            ));
                            info_spans.push(Span::styled(
                                warning.as_str(),
                                Style::default().fg(Color::Yellow),
                            ));
                        } else if let Some(ref meta) = choice.meta {
                            info_spans.push(Span::styled(
                                format!("{} -- {}", choice.label, meta),
                                theme.muted_style,
                            ));
                        }

                        if !info_spans.is_empty() {
                            let info = Paragraph::new(Line::from(info_spans));
                            f.render_widget(info, chunks[offset + 1]);
                        }
                    }
                }
            }
        })?;

        match event::read()? {
            Event::Key(key) => match key.code {
                KeyCode::Up | KeyCode::Char('k') => {
                    let i = state.selected().unwrap_or(0);
                    if i > 0 {
                        state.select(Some(i - 1));
                    }
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    let i = state.selected().unwrap_or(0);
                    if i < choices.len().saturating_sub(1) {
                        state.select(Some(i + 1));
                    }
                }
                KeyCode::Enter => {
                    let idx = state.selected().unwrap_or(default_idx);
                    break Response {
                        result: Some(Value::String(choices[idx].label.clone())),
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
                _ => {}
            },
            Event::Mouse(mouse) => match mouse.kind {
                MouseEventKind::ScrollDown => {
                    let i = state.selected().unwrap_or(0);
                    if i < choices.len().saturating_sub(1) {
                        state.select(Some(i + 1));
                    }
                }
                MouseEventKind::ScrollUp => {
                    let i = state.selected().unwrap_or(0);
                    if i > 0 {
                        state.select(Some(i - 1));
                    }
                }
                _ => {}
            },
            _ => {}
        }
    };

    crossterm::execute!(terminal.backend_mut(), crossterm::cursor::Show)?;
    crossterm::execute!(terminal.backend_mut(), crossterm::event::DisableMouseCapture)?;
    crossterm::execute!(terminal.backend_mut(), crossterm::terminal::LeaveAlternateScreen)?;
    crossterm::terminal::disable_raw_mode()?;
    Ok(result)
}