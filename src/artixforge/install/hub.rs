use crate::contract::Response;
use crate::layout;
use crate::theme::Theme;
use crate::widgets;
use anyhow::Result;
use crossterm::event::{self, Event, KeyCode};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Alignment, Constraint, Layout, Margin},
    style::Modifier,
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph},
    Terminal, Frame,
};
use serde_json::Value;
use std::collections::HashMap;
use std::fs::File;
use std::io;

fn hash_password(password: &str) -> String {
    if password.is_empty() {
        return String::new();
    }
    std::process::Command::new("openssl")
        .args(&["passwd", "-6", "--", password])
        .output()
        .ok()
        .and_then(|o| {
            let s = String::from_utf8_lossy(&o.stdout).trim().to_string();
            if s.is_empty() { None } else { Some(s) }
        })
        .unwrap_or_default()
}

#[derive(Debug, Clone)]
struct HubItem {
    id: String,
    label: String,
    value: String,
    widget: String,
    choices: Vec<String>,
    placeholder: String,
    visible_if: HashMap<String, String>,
    display: String,
    disk_picker: bool,
}

#[derive(Debug, Clone)]
struct HubCategory {
    label: String,
    summary_template: String,
    items: Vec<HubItem>,
}

#[derive(Debug, Clone, PartialEq)]
enum HubMode {
    Browsing,
    EditingItem,
    ConfirmProceed,
}

struct HubState {
    categories: Vec<HubCategory>,
    cat_idx: usize,
    item_idx: usize,
    values: HashMap<String, String>,
    mode: HubMode,
}

#[derive(Clone)]
struct HubRenderData {
    categories: Vec<HubCategory>,
    cat_idx: usize,
    item_idx: usize,
    values: HashMap<String, String>,
    mode: HubMode,
    visible_items: Vec<HubItem>,
    actions: Vec<String>,
    title: String,
}

impl HubRenderData {
    fn render(&self, f: &mut Frame) {
        let theme = Theme::load();
        let area = f.size();

        let box_area = layout::centered(95, 95, area);
        f.render_widget(Clear, box_area);
        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(theme.border_style)
            .title(self.title.as_str())
            .title_style(theme.title_style);
        f.render_widget(block, box_area);

        let inner = box_area.inner(&Margin::new(1, 1));
        let main_chunks = Layout::default()
            .constraints([Constraint::Min(1), Constraint::Length(1)])
            .split(inner);

        let panels = Layout::default()
            .constraints([Constraint::Percentage(30), Constraint::Percentage(70)])
            .split(main_chunks[0]);

        let cat_items: Vec<ListItem> = self
            .categories
            .iter()
            .enumerate()
            .map(|(i, cat)| {
                let is_sel = i == self.cat_idx;
                let style = if is_sel {
                    theme.selected_style
                } else {
                    theme.normal_style
                };
                let summary = format!(
                    " {}",
                    render_summary(&cat.summary_template, &self.values)
                );
                ListItem::new(Line::from(vec![
                    Span::styled(cat.label.clone() + "\n", style.add_modifier(Modifier::BOLD)),
                    Span::styled(summary, theme.muted_style),
                ]))
            })
            .collect();
        let mut cat_list = ListState::default().with_selected(Some(self.cat_idx));
        f.render_stateful_widget(
            List::new(cat_items).highlight_style(theme.selected_style),
            panels[0],
            &mut cat_list,
        );

        let item_lines: Vec<Line> = self
            .visible_items
            .iter()
            .enumerate()
            .map(|(i, item)| {
                let is_sel = i == self.item_idx && self.mode == HubMode::Browsing;
                let style = if is_sel {
                    theme.selected_style
                } else {
                    theme.normal_style
                };
                let val = self.values.get(&item.id).cloned().unwrap_or_default();
                let display_val = if item.display == "set/not set" {
                    if val.is_empty() {
                        "not set".to_string()
                    } else {
                        "set".to_string()
                    }
                } else if item.widget == "user_manager" {
                    if val.is_empty() || val == "0" {
                        "No users".to_string()
                    } else {
                        if let Ok(users) =
                            serde_json::from_str::<Vec<serde_json::Value>>(&val)
                        {
                            if users.is_empty() {
                                "No users".to_string()
                            } else {
                                let first = users[0]
                                    .get("name")
                                    .and_then(|v| v.as_str())
                                    .unwrap_or("?");
                                format!("{} user(s), e.g. {}", users.len(), first)
                            }
                        } else {
                            "Users configured".to_string()
                        }
                    }
                } else if item.disk_picker {
                    if val.is_empty() {
                        "(none)".to_string()
                    } else {
                        val
                    }
                } else if val.is_empty() {
                    "(none)".to_string()
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

        let action_text = self
            .actions
            .iter()
            .enumerate()
            .map(|(i, a)| format!("F{}:{}", i + 1, a))
            .collect::<Vec<_>>()
            .join("  ");
        let footer = format!(
            "{}   j/k:items  h/l:categories  Tab:next  Enter:edit  Esc:cancel",
            action_text
        );
        f.render_widget(
            Paragraph::new(Line::from(Span::styled(footer, theme.muted_style)))
                .alignment(Alignment::Center),
            main_chunks[1],
        );
    }
}

pub fn run(
    terminal: Option<&mut Terminal<CrosstermBackend<File>>>,
    title: String,
    categories_json: Value,
    actions: Vec<String>,
    _boot_mode: String,
) -> Result<Response> {
    let is_daemon = terminal.is_some();
    let theme = Theme::load();

    let mut categories = Vec::new();
    let mut initial_values = HashMap::new();

    if let Some(arr) = categories_json.as_array() {
        for cat_val in arr {
            let label = cat_val["label"].as_str().unwrap_or("").to_string();
            let summary_template = cat_val["summary_template"]
                .as_str()
                .unwrap_or("")
                .to_string();
            let mut items = Vec::new();
            if let Some(items_arr) = cat_val["items"].as_array() {
                for item_val in items_arr {
                    let id = item_val["id"].as_str().unwrap_or("").to_string();
                    if id.is_empty() {
                        continue;
                    }
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
                    items.push(HubItem {
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
                    });
                }
            }
            categories.push(HubCategory {
                label,
                summary_template,
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

        let visible_items: Vec<HubItem> = if !state.categories.is_empty() {
            let cat = &state.categories[state.cat_idx];
            cat.items
                .iter()
                .filter(|item| {
                    if item.visible_if.is_empty() {
                        return true;
                    }
                    item.visible_if
                        .iter()
                        .all(|(k, v)| state.values.get(k).map(|s| s == v).unwrap_or(false))
                })
                .cloned()
                .collect()
        } else {
            vec![]
        };

        if !visible_items.is_empty() {
            state.item_idx = state.item_idx.min(visible_items.len() - 1);
        }

        let render_data = HubRenderData {
            categories: state.categories.clone(),
            cat_idx: state.cat_idx,
            item_idx: state.item_idx,
            values: state.values.clone(),
            mode: state.mode.clone(),
            visible_items: visible_items.clone(),
            actions: actions.clone(),
            title: title.clone(),
        };

        term.draw(|f: &mut Frame| {
            if let Some(ref wm) = theme.watermark_path {
                crate::watermark::render(f, f.size(), wm);
            }
            render_data.render(f);
        })?;

        if state.mode == HubMode::EditingItem {
            if let Some(item) = visible_items.get(state.item_idx).cloned() {
                let current_val = state.values.get(&item.id).cloned().unwrap_or_default();
                let item_id = item.id.clone();
                let result = match item.widget.as_str() {
                    "kernel_picker" => crate::artixforge::install::kernel::run_with_values(
                        term,
                        &current_val,
                        &mut state.values,
                    )
                    .map(|v| Some(v)),
                    "user_manager" => crate::artixforge::install::users::run(term, &current_val),
                    "menu" if item_id == "GUM_TITLE_COLOR" => {
                        let choices = item.choices.clone();
                        let default = if choices.contains(&current_val) {
                            Some(current_val)
                        } else {
                            choices.first().cloned()
                        };
                        let resp = widgets::menu::run(
                            Some(term),
                            item.label.clone(),
                            String::new(),
                            Value::Array(
                                choices
                                    .iter()
                                    .map(|c| Value::String(c.clone()))
                                    .collect(),
                            ),
                            default,
                            None,
                        )?;
                        if resp.cancelled {
                            Ok(None)
                        } else {
                            let name = resp
                                .result
                                .and_then(|v| v.as_str().map(String::from))
                                .unwrap_or_default();
                            let (title_code, accent_code) = match name.as_str() {
                                "Forge (pink/green)" => ("212", "34"),
                                "Artix (blue)" => ("39", "117"),
                                "Jet Black (grey)" => ("245", "250"),
                                "Mono (white)" => ("250", "255"),
                                "Retro (yellow)" => ("3", "11"),
                                _ => ("212", "34"),
                            };
                            state.values.insert(
                                "GUM_TITLE_COLOR".to_string(),
                                title_code.to_string(),
                            );
                            state.values.insert(
                                "GUM_ACCENT_COLOR".to_string(),
                                accent_code.to_string(),
                            );
                            Ok(Some(name))
                        }
                    }
                    "menu" | "disk_picker" => {
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
                            Value::Array(
                                choices
                                    .iter()
                                    .map(|c| Value::String(c.clone()))
                                    .collect(),
                            ),
                            default,
                            None,
                        )?;
                        if resp.cancelled {
                            Ok(None)
                        } else {
                            Ok(resp
                                .result
                                .and_then(|v| v.as_str().map(String::from)))
                        }
                    }
                    "input" => {
                        let bg_fn: &dyn Fn(&mut Frame) = &|f| render_data.render(f);
                        let resp = widgets::input::run_with_background(
                            Some(term),
                            item.label.clone(),
                            String::new(),
                            Some(current_val),
                            Some(item.placeholder.clone()),
                            None,
                            Some(bg_fn),
                        )?;
                        if resp.cancelled {
                            Ok(None)
                        } else {
                            Ok(resp
                                .result
                                .and_then(|v| v.as_str().map(String::from)))
                        }
                    }
                    "filter" => {
                        let choices: Vec<String> = match item_id.as_str() {
                            "TIMEZONE" => get_timezones(),
                            "LOCALE" => get_locales(),
                            "KEYMAP" => get_keymaps(),
                            _ => item.choices.clone(),
                        };
                        let resp = widgets::filter::run(
                            Some(term),
                            item.label.clone(),
                            String::new(),
                            choices,
                            Some(item.placeholder.clone()),
                        )?;
                        if resp.cancelled {
                            Ok(None)
                        } else {
                            Ok(resp
                                .result
                                .and_then(|v| v.as_str().map(String::from)))
                        }
                    }
                    "yesno" => {
                        let default_val = current_val == "yes";
                        let msg = match item_id.as_str() {
                            "SWAP_ENABLED" => "Create a swap partition?\n\nSwap provides virtual memory and hibernation support."
                                .to_string(),
                            "USE_LUKS" => "Encrypt the entire disk with LUKS?\n\nYou will need to enter a passphrase at boot."
                                .to_string(),
                            "USE_LVM" => "Use Logical Volume Management?\n\nAllows flexible partition resizing."
                                .to_string(),
                            "GENERATE_UKI" => "Generate a Unified Kernel Image?\n\nBundles kernel, initramfs, and cmdline into a single EFI file."
                                .to_string(),
                            "ENABLE_ARCH_REPOS" => "Enable official Arch Linux repositories?\n\nRequired for some packages and kernels."
                                .to_string(),
                            "ENABLE_AURIS" => "Enable AURIS?\n\nCommunity-submitted init scripts."
                                .to_string(),
                            "ALLOW_OFFLINE" => "Enable offline mode?\n\nCache all packages for installation without internet."
                                .to_string(),
                            "POWER_USER" => "Enable Power User mode?\n\nBuild packages from source with custom compilation flags."
                                .to_string(),
                            _ => format!("Enable {}?", item.label.to_lowercase()),
                        };
                        let bg_fn: &dyn Fn(&mut Frame) = &|f| render_data.render(f);
                        let resp = widgets::yesno::run_with_background(
                            Some(term),
                            item.label.clone(),
                            msg,
                            Some(default_val),
                            Some(bg_fn),
                        )?;
                        if resp.cancelled {
                            Ok(None)
                        } else {
                            Ok(resp.result.and_then(|v| v.as_bool()).map(|b| {
                                if b {
                                    "yes".to_string()
                                } else {
                                    "no".to_string()
                                }
                            }))
                        }
                    }
                    "password" => {
                        if item_id == "ROOT_PASS" {
                            let bg_fn: &dyn Fn(&mut Frame) = &|f| render_data.render(f);
                            let pass1 = widgets::password::run_with_background(
                                Some(term),
                                "Root Password".into(),
                                "Enter root password:".into(),
                                None,
                                Some(bg_fn),
                            )?;
                            if pass1.cancelled {
                                Ok(None)
                            } else {
                                let bg_fn2: &dyn Fn(&mut Frame) = &|f| render_data.render(f);
                                let pass2 = widgets::password::run_with_background(
                                    Some(term),
                                    "Root Password".into(),
                                    "Confirm password:".into(),
                                    None,
                                    Some(bg_fn2),
                                )?;
                                if pass2.cancelled {
                                    Ok(None)
                                } else {
                                    let p1 = pass1
                                        .result
                                        .and_then(|v| v.as_str().map(String::from));
                                    let p2 = pass2
                                        .result
                                        .and_then(|v| v.as_str().map(String::from));
                                    if p1 == p2 {
                                        Ok(p1.map(|p| hash_password(&p)))
                                    } else {
                                        Ok(None)
                                    }
                                }
                            }
                        } else {
                            let bg_fn: &dyn Fn(&mut Frame) = &|f| render_data.render(f);
                            let resp = widgets::password::run_with_background(
                                Some(term),
                                item.label.clone(),
                                String::new(),
                                Some(if current_val.is_empty() {
                                    "Enter password".into()
                                } else {
                                    "".into()
                                }),
                                Some(bg_fn),
                            )?;
                            if resp.cancelled {
                                Ok(None)
                            } else {
                                Ok(resp
                                    .result
                                    .and_then(|v| v.as_str().map(String::from)))
                            }
                        }
                    }
                    "multiselect" => {
                        let choices: Vec<String> = if item_id == "EXTRAS" {
                            get_extras_choices()
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
                            Ok(resp.result.and_then(|v| v.as_array().cloned()).map(|a| {
                                a.iter()
                                    .filter_map(|v| v.as_str())
                                    .collect::<Vec<_>>()
                                    .join(" ")
                            }))
                        }
                    }
                    _ => Ok(None),
                };

                if let Ok(Some(new_val)) = result {
                    state.values.insert(item_id.clone(), new_val.clone());

                    if item_id == "USE_LUKS" {
                        let bg_fn: &dyn Fn(&mut Frame) = &|f| render_data.render(f);
                        let pass1 = widgets::password::run_with_background(
                            Some(term),
                            "LUKS Passphrase".into(),
                            "Enter passphrase:".into(),
                            None,
                            Some(bg_fn),
                        )?;
                        if !pass1.cancelled {
                            let bg_fn2: &dyn Fn(&mut Frame) = &|f| render_data.render(f);
                            let pass2 = widgets::password::run_with_background(
                                Some(term),
                                "LUKS Passphrase".into(),
                                "Confirm passphrase:".into(),
                                None,
                                Some(bg_fn2),
                            )?;
                            if !pass2.cancelled {
                                let p1 = pass1
                                    .result
                                    .and_then(|v| v.as_str().map(String::from));
                                let p2 = pass2
                                    .result
                                    .and_then(|v| v.as_str().map(String::from));
                                if p1 == p2 {
                                    if let Some(pass) = p1 {
                                        state.values.insert("LUKS_PASS".to_string(), pass);
                                        let bg_fn3: &dyn Fn(&mut Frame) = &|f| render_data.render(f);
                                        let kf_resp = widgets::yesno::run_with_background(
                                            Some(term),
                                            "LUKS Keyfile".into(),
                                            "Use a keyfile to avoid typing your password twice at boot?"
                                                .into(),
                                            Some(false),
                                            Some(bg_fn3),
                                        )?;
                                        state.values.insert(
                                            "LUKS_KEYFILE".to_string(),
                                            if kf_resp
                                                .result
                                                .and_then(|v| v.as_bool())
                                                .unwrap_or(false)
                                            {
                                                "yes".to_string()
                                            } else {
                                                "no".to_string()
                                            },
                                        );
                                    }
                                }
                            }
                        }
                    }
                }
            }
            state.mode = HubMode::Browsing;
            continue;
        }

        if state.mode == HubMode::ConfirmProceed {
            let proceed_bg: &dyn Fn(&mut Frame) = &|f| render_data.render(f);
            let resp = widgets::yesno::run_with_background(
                Some(term),
                "Proceed".into(),
                "Begin installation with these settings?".into(),
                Some(true),
                Some(proceed_bg),
            )?;
            if resp.cancelled {
                state.mode = HubMode::Browsing;
            } else if resp.result.and_then(|v| v.as_bool()).unwrap_or(false) {
                if let Some(disk) = state.values.get("DISK") {
                    let short_disk = disk.split_whitespace().next().unwrap_or(disk).to_string();
                    state.values.insert("DISK".to_string(), short_disk);
                }
                let map: serde_json::Map<String, Value> = state
                    .values
                    .iter()
                    .filter(|(k, _)| !k.is_empty())
                    .map(|(k, v)| (k.clone(), Value::String(v.clone())))
                    .collect();
                break Response {
                    result: Some(Value::Object(map)),
                    cancelled: false,
                    error: None,
                };
            } else {
                state.mode = HubMode::Browsing;
            }
            continue;
        }

        match event::read()? {
            Event::Key(key) => match key.code {
                KeyCode::Esc => break Response {
                    result: None,
                    cancelled: true,
                    error: None,
                },
                KeyCode::Up | KeyCode::Char('k') => {
                    state.item_idx = state.item_idx.saturating_sub(1);
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    if state.item_idx + 1 < visible_items.len() {
                        state.item_idx += 1;
                    }
                }
                KeyCode::Left | KeyCode::Char('h') => {
                    if state.cat_idx > 0 {
                        state.cat_idx -= 1;
                    } else {
                        state.cat_idx = state.categories.len().saturating_sub(1);
                    }
                    state.item_idx = 0;
                }
                KeyCode::Right | KeyCode::Char('l') | KeyCode::Tab => {
                    if state.cat_idx + 1 < state.categories.len() {
                        state.cat_idx += 1;
                    } else {
                        state.cat_idx = 0;
                    }
                    state.item_idx = 0;
                }
                KeyCode::Enter => {
                    state.mode = HubMode::EditingItem;
                }
                KeyCode::F(f) if f >= 1 && f <= actions.len() as u8 => {
                    let action = &actions[f as usize - 1];
                    if action == "Proceed" {
                        state.mode = HubMode::ConfirmProceed;
                    } else if action == "Quick Profile" {
                        if let Ok(Some(profile_state)) =
                            crate::artixforge::install::quick_profiles::run(term)
                        {
                            for (k, v) in profile_state {
                                state.values.insert(k, v);
                            }
                        }
                    } else {
                        let mut map: serde_json::Map<String, Value> = state
                            .values
                            .iter()
                            .filter(|(k, _)| !k.is_empty())
                            .map(|(k, v)| (k.clone(), Value::String(v.clone())))
                            .collect();
                        map.insert(
                            "_action".to_string(),
                            Value::String(action.clone()),
                        );
                        break Response {
                            result: Some(Value::Object(map)),
                            cancelled: false,
                            error: None,
                        };
                    }
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
        if key == "USER_COUNT" {
            let count = if value == "0" || value.is_empty() {
                0
            } else if let Ok(users) = serde_json::from_str::<Vec<serde_json::Value>>(value) {
                users.len()
            } else {
                0
            };
            result = result.replace(&format!("{{{}}}", key), &count.to_string());
        } else {
            result = result.replace(&format!("{{{}}}", key), value);
        }
    }
    result
}

fn get_disks() -> Vec<String> {
    if let Ok(output) = std::process::Command::new("lsblk")
        .args(&["-dpno", "NAME,SIZE,MODEL", "-e", "7"])
        .output()
    {
        let s = String::from_utf8_lossy(&output.stdout);
        s.lines()
            .map(|l| {
                let parts: Vec<&str> = l.splitn(3, ' ').collect();
                let name = parts.first().unwrap_or(&"");
                let size = parts.get(1).unwrap_or(&"");
                let model = parts.get(2).unwrap_or(&"Unknown");
                format!("{} - {} ({})", name, size, model)
            })
            .collect()
    } else {
        vec!["/dev/sda - 0 (Unknown)".into()]
    }
}

fn get_timezones() -> Vec<String> {
    std::process::Command::new("sh")
        .arg("-c")
        .arg("find /usr/share/zoneinfo -type f 2>/dev/null | sed 's|/usr/share/zoneinfo/||' | grep -v '^posix\\|^right\\|^Etc\\|\\.tab$' | sort")
        .output()
        .map(|o| {
            String::from_utf8_lossy(&o.stdout)
                .lines()
                .map(|l| l.to_string())
                .collect()
        })
        .unwrap_or_default()
}

fn get_locales() -> Vec<String> {
    std::process::Command::new("sh")
        .arg("-c")
        .arg("grep -E '^#?[a-z]{2}_[A-Z]{2}.*UTF-8' /etc/locale.gen 2>/dev/null | sed 's/^#//' | awk '{print $1}' | sort -u")
        .output()
        .map(|o| {
            String::from_utf8_lossy(&o.stdout)
                .lines()
                .map(|l| l.to_string())
                .collect()
        })
        .unwrap_or_default()
}

fn get_keymaps() -> Vec<String> {
    std::process::Command::new("sh")
        .arg("-c")
        .arg("localectl list-keymaps 2>/dev/null || find /usr/share/kbd/keymaps -name '*.map.gz' 2>/dev/null | sed 's|.*/||; s|\\.map\\.gz||' | sort -u")
        .output()
        .map(|o| {
            String::from_utf8_lossy(&o.stdout)
                .lines()
                .map(|l| l.to_string())
                .collect()
        })
        .unwrap_or_default()
}

fn get_extras_choices() -> Vec<String> {
    let output = std::process::Command::new("pacman")
        .args(&["-Sl", "world", "galaxy"])
        .output();
    
    match output {
        Ok(o) => {
            let stdout = String::from_utf8_lossy(&o.stdout);
            let mut packages: Vec<String> = stdout
                .lines()
                .filter_map(|line| line.split_whitespace().nth(1))
                .map(|s| s.to_string())
                .collect();
            packages.sort();
            packages.dedup();
            packages
        }
        Err(_) => vec![]
    }
}