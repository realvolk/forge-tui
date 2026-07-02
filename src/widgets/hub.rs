use crate::contract::Response;
use crate::layout;
use crate::theme::Theme;
use crate::widgets::{self, helpers};
use anyhow::Result;
use crossterm::event::{self, Event, KeyCode};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Alignment, Constraint, Layout, Margin, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph},
    Terminal, Frame,
};
use serde_json::Value;
use std::collections::HashMap;
use std::fs::File;
use std::io;

#[derive(Debug, Clone)]
struct Item {
    id: String,
    label: String,
    value: String,
    widget: String,
    choices: Vec<String>,
    placeholder: String,
    visible_if: HashMap<String, String>,
    display: String,
    disk_picker: bool,
    choices_from: String,
}

#[derive(Debug, Clone)]
struct Category {
    id: String,
    label: String,
    summary_template: String,
    items: Vec<Item>,
}

#[derive(Debug, Clone, PartialEq)]
enum HubMode {
    Browsing,
    EditingItem,
    ConfirmQuit,
}

struct HubState {
    categories: Vec<Category>,
    cat_idx: usize,
    item_idx: usize,
    values: HashMap<String, String>,
    mode: HubMode,
    edit_buffer: String,
    edit_list_state: ListState,
}

pub fn run(
    terminal: Option<&mut Terminal<CrosstermBackend<File>>>,
    title: String,
    categories_json: Value,
    actions: Vec<String>,
) -> Result<Response> {
    let is_daemon = terminal.is_some();
    let theme = Theme::load();

    let mut categories = Vec::new();
    let mut initial_values = HashMap::new();

    if let Some(arr) = categories_json.as_array() {
        for cat_val in arr {
            let cat_id = cat_val["id"].as_str().unwrap_or("").to_string();
            let cat_label = cat_val["label"].as_str().unwrap_or("").to_string();
            let cat_summary = cat_val["summary_template"]
                .as_str()
                .unwrap_or("")
                .to_string();
            let mut items = Vec::new();

            if let Some(items_arr) = cat_val["items"].as_array() {
                for item_val in items_arr {
                    let id = item_val["id"].as_str().unwrap_or("").to_string();
                    let value = item_val["value"].as_str().unwrap_or("").to_string();
                    let visible_if: HashMap<String, String> = item_val
                        .get("visible_if")
                        .and_then(|v| v.as_object())
                        .map(|o| {
                            o.iter()
                                .map(|(k, v)| (k.clone(), v.as_str().unwrap_or("").to_string()))
                                .collect()
                        })
                        .unwrap_or_default();

                    initial_values.insert(id.clone(), value.clone());

                    items.push(Item {
                        id,
                        label: item_val["label"].as_str().unwrap_or("").to_string(),
                        value,
                        widget: item_val["widget"].as_str().unwrap_or("menu").to_string(),
                        choices: item_val["choices"]
                            .as_array()
                            .map(|a| {
                                a.iter()
                                    .filter_map(|v| v.as_str().map(String::from))
                                    .collect()
                            })
                            .unwrap_or_default(),
                        placeholder: item_val["placeholder"]
                            .as_str()
                            .unwrap_or("")
                            .to_string(),
                        visible_if,
                        display: item_val["display"].as_str().unwrap_or("").to_string(),
                        disk_picker: item_val["disk_picker"].as_bool().unwrap_or(false),
                        choices_from: item_val["choices_from"]
                            .as_str()
                            .unwrap_or("")
                            .to_string(),
                    });
                }
            }
            categories.push(Category {
                id: cat_id,
                label: cat_label,
                summary_template: cat_summary,
                items,
            });
        }
    }

    let mut state = HubState {
        categories,
        cat_idx: 0,
        item_idx: 0,
        values: initial_values,
        mode: HubMode::Browsing,
        edit_buffer: String::new(),
        edit_list_state: ListState::default(),
    };

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
        if !state.categories.is_empty() {
            state.cat_idx = state.cat_idx.min(state.categories.len() - 1);
        }

        let visible_items: Vec<&Item> = if !state.categories.is_empty() {
            let cat = &state.categories[state.cat_idx];
            cat.items
                .iter()
                .filter(|item| {
                    if item.visible_if.is_empty() {
                        return true;
                    }
                    item.visible_if.iter().all(|(k, v)| {
                        state.values.get(k).map(|s| s == v).unwrap_or(false)
                    })
                })
                .collect()
        } else {
            vec![]
        };

        if !visible_items.is_empty() {
            state.item_idx = state.item_idx.min(visible_items.len() - 1);
        }

        term.draw(|f: &mut Frame| {
            let area = f.size();
            if let Some(ref wm) = theme.watermark_path {
                crate::watermark::render(f, area, wm);
            }

            let box_area = layout::centered(95, 95, area);
            f.render_widget(Clear, box_area);
            let block = Block::default()
                .borders(Borders::ALL)
                .border_style(theme.border_style)
                .title(title.as_str())
                .title_style(theme.title_style);
            f.render_widget(block, box_area);

            let inner = box_area.inner(&Margin::new(1, 1));
            let main_chunks = Layout::default()
                .constraints([Constraint::Min(1), Constraint::Length(1)])
                .split(inner);

            let panels = Layout::default()
                .constraints([Constraint::Percentage(30), Constraint::Percentage(70)])
                .split(main_chunks[0]);

            /* Left panel – categories */
            let cat_items: Vec<ListItem> = state
                .categories
                .iter()
                .enumerate()
                .map(|(i, cat)| {
                    let is_sel = i == state.cat_idx;
                    let style = if is_sel {
                        theme.selected_style
                    } else {
                        theme.normal_style
                    };
                    let summary = render_summary(&cat.summary_template, &state.values);
                    ListItem::new(Line::from(vec![
                        Span::styled(
                            cat.label.clone() + "\n",
                            style.add_modifier(Modifier::BOLD),
                        ),
                        Span::styled(summary, theme.muted_style),
                    ]))
                })
                .collect();

            let mut cat_list_state = ListState::default().with_selected(Some(state.cat_idx));
            f.render_stateful_widget(
                List::new(cat_items).highlight_style(theme.selected_style),
                panels[0],
                &mut cat_list_state,
            );

            /* Right panel – items */
            let item_lines: Vec<Line> = visible_items
                .iter()
                .enumerate()
                .map(|(i, item)| {
                    let is_sel = i == state.item_idx && state.mode == HubMode::Browsing;
                    let style = if is_sel {
                        theme.selected_style
                    } else {
                        theme.normal_style
                    };
                    let val = state.values.get(&item.id).cloned().unwrap_or_default();
                    let display_val = if item.display == "set/not set" {
                        if val.is_empty() {
                            "not set".to_string()
                        } else {
                            "set".to_string()
                        }
                    } else if item.widget == "disk_picker" {
                        let short = val.rsplit('/').next().unwrap_or(&val).to_string();
                        format!("{} ({})", short, val)
                    } else {
                        val
                    };
                    Line::from(vec![
                        Span::styled(format!(" {}: ", item.label), style),
                        Span::styled(display_val, theme.accent_style),
                    ])
                })
                .collect();

            f.render_widget(
                Paragraph::new(item_lines).block(Block::default().borders(Borders::LEFT)),
                panels[1],
            );

            /* Footer with actions and hints */
            let action_text = actions
                .iter()
                .enumerate()
                .map(|(i, a)| format!("F{}:{}", i + 1, a))
                .collect::<Vec<_>>()
                .join("  ");
            let footer = format!(
                "{}   j/k:nav  Tab:switch  Enter:edit  Esc:quit",
                action_text
            );
            f.render_widget(
                Paragraph::new(Line::from(Span::styled(footer, theme.muted_style)))
                    .alignment(Alignment::Center),
                main_chunks[1],
            );
        })?;

        match state.mode {
            HubMode::EditingItem => {
                if let Some(item) = visible_items.get(state.item_idx).cloned() {
                    let result = dispatch_item_edit(term, &item, &state.values);
                    match result {
                        Ok(Some(new_val)) => {
                            state.values.insert(item.id.clone(), new_val);
                        }
                        _ => {}
                    }
                    state.mode = HubMode::Browsing;
                }
                continue;
            }
            HubMode::ConfirmQuit => {
                if let Event::Key(key) = event::read()? {
                    match key.code {
                        KeyCode::Char('y') | KeyCode::Char('Y') => {
                            break Response {
                                result: None,
                                cancelled: true,
                                error: None,
                            };
                        }
                        _ => state.mode = HubMode::Browsing,
                    }
                }
                continue;
            }
            HubMode::Browsing => {}
        }

        match event::read()? {
            Event::Key(key) => match key.code {
                KeyCode::Esc => state.mode = HubMode::ConfirmQuit,
                KeyCode::Up | KeyCode::Char('k') => {
                    state.item_idx = state.item_idx.saturating_sub(1);
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    if state.item_idx + 1 < visible_items.len() {
                        state.item_idx += 1;
                    }
                }
                KeyCode::Left | KeyCode::Char('h') => {
                    state.cat_idx = state.cat_idx.saturating_sub(1);
                    state.item_idx = 0;
                }
                KeyCode::Right | KeyCode::Char('l') | KeyCode::Tab => {
                    if state.cat_idx + 1 < state.categories.len() {
                        state.cat_idx += 1;
                        state.item_idx = 0;
                    }
                }
                KeyCode::Enter => {
                    if let Some(item) = visible_items.get(state.item_idx).cloned() {
                        state.mode = HubMode::EditingItem;
                    }
                }
                KeyCode::F(f) if f >= 1 && f <= actions.len() as u8 => {
                    if f == actions.len() as u8 {
                        let map: serde_json::Map<String, Value> = state
                            .values
                            .iter()
                            .map(|(k, v)| (k.clone(), Value::String(v.clone())))
                            .collect();
                        break Response {
                            result: Some(Value::Object(map)),
                            cancelled: false,
                            error: None,
                        };
                    }
                    break Response {
                        result: Some(Value::String(actions[f as usize - 1].clone())),
                        cancelled: false,
                        error: None,
                    };
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

fn render_summary(template: &str, values: &HashMap<String, String>) -> String {
    let mut result = template.to_string();
    for (key, value) in values {
        let placeholder = format!("{{{}}}", key);
        result = result.replace(&placeholder, value);
    }
    result
}

fn dispatch_item_edit(
    term: &mut Terminal<CrosstermBackend<File>>,
    item: &Item,
    values: &HashMap<String, String>,
) -> Result<Option<String>> {
    let current_val = values.get(&item.id).cloned().unwrap_or_default();

    match item.widget.as_str() {
        "menu" | "kernel_picker" | "disk_picker" => {
            let choices: Vec<String> = if item.disk_picker {
                get_disks()
            } else {
                item.choices.clone()
            };
            let default = if choices.contains(&current_val) {
                Some(current_val)
            } else {
                choices.first().cloned()
            };

            let resp = widgets::menu::run(
                Some(term),
                item.label.clone(),
                String::new(),
                Value::Array(choices.iter().map(|c| Value::String(c.clone())).collect()),
                default,
                None,
            )?;

            if resp.cancelled {
                Ok(None)
            } else {
                Ok(resp.result.and_then(|v| v.as_str().map(String::from)))
            }
        }

        "input" => {
            let resp = widgets::input::run(
                Some(term),
                item.label.clone(),
                String::new(),
                Some(current_val),
                Some(item.placeholder.clone()),
                None,
            )?;
            if resp.cancelled {
                Ok(None)
            } else {
                Ok(resp.result.and_then(|v| v.as_str().map(String::from)))
            }
        }

        "yesno" => {
            let default_val = current_val == "yes";
            let resp = widgets::yesno::run(
                Some(term),
                item.label.clone(),
                String::new(),
                Some(default_val),
            )?;
            if resp.cancelled {
                Ok(None)
            } else {
                let val = resp
                    .result
                    .and_then(|v| v.as_bool())
                    .map(|b| if b { "yes" } else { "no" });
                Ok(val.map(String::from))
            }
        }

        "password" => {
            let resp = widgets::password::run(
                Some(term),
                item.label.clone(),
                String::new(),
                Some(if current_val.is_empty() {
                    "Enter password".into()
                } else {
                    "".into()
                }),
            )?;
            if resp.cancelled {
                Ok(None)
            } else {
                Ok(resp.result.and_then(|v| v.as_str().map(String::from)))
            }
        }

        "multiselect" => {
            let choices = if !item.choices_from.is_empty() {
                vec!["placeholder".into()]
            } else {
                item.choices.clone()
            };
            let resp = widgets::multiselect::run(
                Some(term),
                item.label.clone(),
                String::new(),
                choices,
                Some("Search...".into()),
                None,
                None,
            )?;
            if resp.cancelled {
                Ok(None)
            } else {
                let arr = resp.result.and_then(|v| v.as_array().cloned());
                let joined = arr.map(|a| {
                    a.iter()
                        .filter_map(|v| v.as_str())
                        .collect::<Vec<&str>>()
                        .join(" ")
                });
                Ok(joined)
            }
        }

        "disk" => {
            let partitions = serde_json::Value::Array(vec![]);
            let free_space = serde_json::Value::Array(vec![serde_json::json!({
                "start": "0",
                "end": "0",
                "size": "0"
            })]);
            let resp = widgets::disk::run(
                Some(term),
                item.label.clone(),
                item.value.clone(),
                partitions,
                Some(free_space),
                None,
            )?;
            if resp.cancelled {
                Ok(None)
            } else {
                Ok(resp.result.map(|v| v.to_string()))
            }
        }

        _ => Ok(None),
    }
}

fn get_disks() -> Vec<String> {
    if let Ok(output) = std::process::Command::new("lsblk")
        .args(&["-dpno", "NAME"])
        .output()
    {
        let s = String::from_utf8_lossy(&output.stdout);
        s.lines().map(|l| l.trim().to_string()).collect()
    } else {
        vec!["/dev/sda".into(), "/dev/nvme0n1".into()]
    }
}