use crate::theme::Theme;
use crate::widgets::{self, helpers};
use anyhow::Result;
use crossterm::event::{self, Event, KeyCode};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Layout, Margin},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph, Wrap},
    Terminal, Frame,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::fs::File;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct User {
    name: String,
    pass: String,
    shell: String,
    groups: Vec<String>,
    sudo: bool,
}

#[derive(Debug, Clone, PartialEq)]
enum UserMode {
    List,
    AddEdit(usize),
    DeleteConfirm(usize),
}

pub fn run(
    term: &mut Terminal<CrosstermBackend<File>>,
    current_value: &str,
) -> Result<Option<String>> {
    let theme = Theme::load();

    let mut users: Vec<User> = if !current_value.is_empty() && current_value != "0" {
        serde_json::from_str(current_value).unwrap_or_default()
    } else {
        vec![]
    };

    let mut mode = UserMode::List;
    let mut list_state = ListState::default();
    let mut edit_name = String::new();
    let mut edit_pass = String::new();
    let mut edit_shell_idx: usize = 0;
    let mut edit_groups: Vec<bool> = vec![];
    let mut edit_sudo = true;
    let mut edit_field: usize = 0;

    let shells: Vec<String> = vec![
        "/bin/bash".into(),
        "/bin/zsh".into(),
        "/usr/bin/fish".into(),
    ];
    let all_groups: Vec<String> = vec![
        "wheel".into(), "audio".into(), "video".into(), "storage".into(),
        "lp".into(), "network".into(), "optical".into(), "scanner".into(),
        "users".into(),
    ];

    let result = loop {
        term.draw(|f: &mut Frame| {
            let area = f.size();
            if let Some(ref wm) = theme.watermark_path {
                crate::watermark::render(f, area, wm);
            }

            let box_area = crate::layout::centered(70, 75, area);
            f.render_widget(Clear, box_area);
            let block = Block::default()
                .borders(Borders::ALL)
                .border_style(theme.border_style)
                .title("User Management")
                .title_style(theme.title_style);
            f.render_widget(block, box_area);
            let inner = box_area.inner(&Margin::new(2, 1));

            match &mode {
                UserMode::List => {
                    let chunks = Layout::default()
                        .constraints([Constraint::Min(1), Constraint::Length(1)])
                        .split(inner);

                    let items: Vec<ListItem> = if users.is_empty() {
                        vec![ListItem::new(Line::from(Span::styled(
                            "  No users yet",
                            theme.muted_style,
                        )))]
                    } else {
                        users
                            .iter()
                            .enumerate()
                            .map(|(i, u)| {
                                let style = if Some(i) == list_state.selected() {
                                    theme.selected_style
                                } else {
                                    theme.normal_style
                                };
                                let sudo_str = if u.sudo { "sudo" } else { "nosudo" };
                                ListItem::new(Line::from(vec![Span::styled(
                                    format!(" {} ({})", u.name, sudo_str),
                                    style,
                                )]))
                            })
                            .collect()
                    };
                    f.render_stateful_widget(
                        List::new(items)
                            .highlight_style(theme.selected_style)
                            .highlight_symbol("> "),
                        chunks[0],
                        &mut list_state,
                    );

                    f.render_widget(
                        helpers::footer("A:add  E:edit  D:delete  Enter:done  Esc:cancel"),
                        chunks[1],
                    );
                }

                UserMode::AddEdit(idx) => {
                    let is_new = *idx >= users.len();
                    let user = if is_new {
                        &User {
                            name: edit_name.clone(),
                            pass: String::new(),
                            shell: shells[edit_shell_idx].clone(),
                            groups: edit_groups
                                .iter()
                                .enumerate()
                                .filter(|(_, &b)| b)
                                .map(|(i, _)| all_groups[i].clone())
                                .collect(),
                            sudo: edit_sudo,
                        }
                    } else {
                        &users[*idx]
                    };

                    let field_names = vec!["Name", "Password", "Shell", "Groups", "Sudo"];
                    let field_values: Vec<String> = vec![
                        if is_new { edit_name.clone() } else { user.name.clone() },
                        "••••".to_string(),
                        if is_new { shells[edit_shell_idx].clone() } else { user.shell.clone() },
                        if is_new {
                            edit_groups
                                .iter()
                                .enumerate()
                                .filter(|(_, &b)| b)
                                .map(|(i, _)| all_groups[i].clone())
                                .collect::<Vec<_>>()
                                .join(", ")
                        } else {
                            user.groups.join(", ")
                        },
                        if is_new {
                            if edit_sudo { "yes".to_string() } else { "no".to_string() }
                        } else {
                            if user.sudo { "yes".to_string() } else { "no".to_string() }
                        },
                    ];

                    let fields: Vec<Line> = field_names
                        .iter()
                        .enumerate()
                        .map(|(i, name)| {
                            let is_sel = i == edit_field;
                            let style = if is_sel { theme.selected_style } else { theme.normal_style };
                            Line::from(vec![
                                Span::styled(format!(" {}: ", name), style),
                                Span::styled(&field_values[i], theme.accent_style),
                            ])
                        })
                        .collect();

                    let chunks = Layout::default()
                        .constraints([Constraint::Min(1), Constraint::Length(1)])
                        .split(inner);
                    f.render_widget(Paragraph::new(fields), chunks[0]);
                    f.render_widget(
                        helpers::footer("j/k:field  Enter:edit  S:save  Esc:back"),
                        chunks[1],
                    );
                }

                UserMode::DeleteConfirm(idx) => {
                    let user = &users[*idx];
                    let msg = format!("Delete user '{}'?\n\nThis cannot be undone.", user.name);
                    let chunks = Layout::default()
                        .constraints([Constraint::Min(1), Constraint::Length(1)])
                        .split(inner);
                    f.render_widget(
                        Paragraph::new(msg).style(theme.normal_style).wrap(Wrap { trim: false }),
                        chunks[0],
                    );
                    f.render_widget(helpers::footer("Y:yes  N:no"), chunks[1]);
                }
            }
        })?;

        match &mode {
            UserMode::List => {
                if let Event::Key(key) = event::read()? {
                    match key.code {
                        KeyCode::Esc | KeyCode::Enter => break,
                        KeyCode::Up | KeyCode::Char('k') => {
                            let i = list_state.selected().unwrap_or(0);
                            if i > 0 { list_state.select(Some(i - 1)); }
                        }
                        KeyCode::Down | KeyCode::Char('j') => {
                            let i = list_state.selected().unwrap_or(0);
                            if i < users.len().saturating_sub(1) { list_state.select(Some(i + 1)); }
                        }
                        KeyCode::Char('a') => {
                            edit_name = String::new();
                            edit_pass = String::new();
                            edit_shell_idx = 0;
                            edit_groups = vec![false; all_groups.len()];
                            edit_sudo = true;
                            edit_field = 0;
                            mode = UserMode::AddEdit(users.len());
                        }
                        KeyCode::Char('e') => {
                            if let Some(idx) = list_state.selected() {
                                if idx < users.len() {
                                    let u = &users[idx];
                                    edit_name = u.name.clone();
                                    edit_pass = String::new();
                                    edit_shell_idx = shells.iter().position(|s| s == &u.shell).unwrap_or(0);
                                    edit_groups = all_groups.iter().map(|g| u.groups.contains(g)).collect();
                                    edit_sudo = u.sudo;
                                    edit_field = 0;
                                    mode = UserMode::AddEdit(idx);
                                }
                            }
                        }
                        KeyCode::Char('d') => {
                            if let Some(idx) = list_state.selected() {
                                if idx < users.len() {
                                    mode = UserMode::DeleteConfirm(idx);
                                }
                            }
                        }
                        _ => {}
                    }
                }
            }

            UserMode::AddEdit(idx) => {
                let is_new = *idx >= users.len();
                if let Event::Key(key) = event::read()? {
                    match key.code {
                        KeyCode::Esc => { mode = UserMode::List; }
                        KeyCode::Char('s') => {
                            let user = User {
                                name: edit_name.clone(),
                                pass: edit_pass.clone(),
                                shell: shells[edit_shell_idx].clone(),
                                groups: edit_groups
                                    .iter()
                                    .enumerate()
                                    .filter(|(_, &b)| b)
                                    .map(|(i, _)| all_groups[i].clone())
                                    .collect(),
                                sudo: edit_sudo,
                            };
                            if is_new { users.push(user); } else { users[*idx] = user; }
                            mode = UserMode::List;
                        }
                        KeyCode::Up | KeyCode::Char('k') => { edit_field = edit_field.saturating_sub(1); }
                        KeyCode::Down | KeyCode::Char('j') => { if edit_field < 4 { edit_field += 1; } }
                        KeyCode::Enter => {
                            match edit_field {
                                0 => {
                                    let resp = widgets::input::run(
                                        Some(term), "Username".into(), String::new(),
                                        Some(edit_name.clone()), None, None,
                                    )?;
                                    if !resp.cancelled {
                                        edit_name = resp.result.and_then(|v| v.as_str().map(String::from)).unwrap_or_default();
                                    }
                                }
                                1 => {
                                    let resp = widgets::password::run(
                                        Some(term), "Password".into(), String::new(),
                                        Some("Enter password".into()),
                                    )?;
                                    if !resp.cancelled {
                                        edit_pass = resp.result.and_then(|v| v.as_str().map(String::from)).unwrap_or_default();
                                    }
                                }
                                2 => {
                                    let resp = widgets::menu::run(
                                        Some(term), "Shell".into(), String::new(),
                                        Value::Array(shells.iter().map(|s| Value::String(s.clone())).collect()),
                                        Some(shells[edit_shell_idx].clone()), None,
                                    )?;
                                    if !resp.cancelled {
                                        if let Some(val) = resp.result.and_then(|v| v.as_str().map(String::from)) {
                                            edit_shell_idx = shells.iter().position(|s| s == &val).unwrap_or(0);
                                        }
                                    }
                                }
                                3 => {
                                    let resp = widgets::checklist::run(
                                        Some(term), "Groups".into(), String::new(),
                                        all_groups.clone(), None, None, None,
                                        Some(
                                            all_groups.iter().enumerate()
                                                .filter(|(i, _)| edit_groups.get(*i).copied().unwrap_or(false))
                                                .map(|(_, g)| g.clone()).collect(),
                                        ),
                                    )?;
                                    if !resp.cancelled {
                                        if let Some(arr) = resp.result.and_then(|v| v.as_array().cloned()) {
                                            let selected: Vec<String> = arr.iter().filter_map(|v| v.as_str().map(String::from)).collect();
                                            edit_groups = all_groups.iter().map(|g| selected.contains(g)).collect();
                                        }
                                    }
                                }
                                4 => {
                                    let resp = widgets::yesno::run(
                                        Some(term), "Sudo Access".into(),
                                        "Grant sudo privileges?".into(), Some(edit_sudo),
                                    )?;
                                    if !resp.cancelled {
                                        edit_sudo = resp.result.and_then(|v| v.as_bool()).unwrap_or(edit_sudo);
                                    }
                                }
                                _ => {}
                            }
                        }
                        _ => {}
                    }
                }
            }

            UserMode::DeleteConfirm(idx) => {
                if let Event::Key(key) = event::read()? {
                    match key.code {
                        KeyCode::Char('y') | KeyCode::Char('Y') => { users.remove(*idx); mode = UserMode::List; }
                        KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => { mode = UserMode::List; }
                        _ => {}
                    }
                }
            }
        }
    };

    let json = serde_json::to_string(&users).unwrap_or_else(|_| "[]".to_string());
    Ok(Some(json))
}