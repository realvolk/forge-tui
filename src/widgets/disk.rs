use crate::contract::Response;
use crate::layout;
use crate::theme::Theme;
use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyModifiers, MouseEventKind};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Alignment, Constraint, Layout, Margin, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
    Terminal, Frame,
};
use serde_json::Value;
use std::collections::HashMap;
use std::fs::File;
use std::io;

#[derive(Debug, Clone)]
struct Partition {
    number: u32,
    start: String,
    end: String,
    size: String,
    ptype: String,
    flags: Vec<String>,
    fs_signature: Option<String>,
}

#[derive(Debug, Clone)]
struct FreeSpace {
    start: String,
    end: String,
    size: String,
}

fn parse_partitions(json: &Value) -> Vec<Partition> {
    let mut parts = Vec::new();
    if let Some(arr) = json.as_array() {
        for v in arr {
            let flags: Vec<String> = v.get("flags")
                .and_then(|f| f.as_array())
                .map(|a| a.iter().filter_map(|s| s.as_str().map(String::from)).collect())
                .unwrap_or_default();
            parts.push(Partition {
                number: v.get("number").and_then(|n| n.as_u64()).unwrap_or(0) as u32,
                start: v.get("start").and_then(|s| s.as_str()).unwrap_or("").to_string(),
                end: v.get("end").and_then(|s| s.as_str()).unwrap_or("").to_string(),
                size: v.get("size").and_then(|s| s.as_str()).unwrap_or("").to_string(),
                ptype: v.get("type").and_then(|s| s.as_str()).unwrap_or("").to_string(),
                flags,
                fs_signature: v.get("fs_signature").and_then(|s| s.as_str()).map(String::from),
            });
        }
    }
    parts.sort_by_key(|p| p.number);
    parts
}

fn parse_free_space(json: &Value) -> Vec<FreeSpace> {
    let mut free = Vec::new();
    if let Some(arr) = json.as_array() {
        for v in arr {
            free.push(FreeSpace {
                start: v.get("start").and_then(|s| s.as_str()).unwrap_or("").to_string(),
                end: v.get("end").and_then(|s| s.as_str()).unwrap_or("").to_string(),
                size: v.get("size").and_then(|s| s.as_str()).unwrap_or("").to_string(),
            });
        }
    }
    free
}

fn partition_type_choices() -> Vec<&'static str> {
    vec![
        "EFI System",
        "Linux filesystem",
        "Linux swap",
        "Linux LVM",
        "Linux LUKS",
        "BIOS boot",
        "Windows data",
        "FreeBSD",
        "Custom",
    ]
}

fn flag_choices() -> Vec<&'static str> {
    vec!["boot", "esp", "bios_grub", "lvm", "raid"]
}

pub fn run(
    terminal: Option<&mut Terminal<CrosstermBackend<File>>>,
    title: String,
    disk: String,
    partitions_json: Value,
    free_space_json: Option<Value>,
    readonly: Option<bool>,
) -> Result<Response> {
    let is_daemon = terminal.is_some();
    let readonly = readonly.unwrap_or(false);
    let theme = Theme::load();

    let mut partitions = parse_partitions(&partitions_json);
    let mut free_space = parse_free_space(&free_space_json.unwrap_or(Value::Null));
    let mut selected_idx: usize = 0;
    let mut scroll: u16 = 0;
    let mut show_confirm: Option<ConfirmDialog> = None;

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

    let result = loop {
        // Clamp selection
        let total_items = partitions.len() + free_space.len();
        if total_items > 0 {
            selected_idx = selected_idx.min(total_items - 1);
        }

        // Draw
        terminal.draw(|f: &mut Frame| {
            let area = f.size();
            if let Some(ref wm) = theme.watermark_path {
                crate::watermark::render(f, area, wm);
            }

            let box_area = layout::centered(95, 90, area);
            f.render_widget(Clear, box_area);

            let block = Block::default()
                .borders(Borders::ALL)
                .border_style(theme.border_style)
                .title(format!("{} ({})", title, disk))
                .title_style(theme.title_style);
            f.render_widget(block, box_area);

            let inner = box_area.inner(&Margin::new(1, 1));
            let chunks = Layout::default()
                .constraints([
                    Constraint::Length(5),  // partition map bar
                    Constraint::Min(1),     // partition list
                    Constraint::Length(2),  // detail line
                    Constraint::Length(1),  // action bar
                ])
                .split(inner);

            // --- Partition map bar ---
            draw_partition_bar(f, chunks[0], &partitions, &free_space, &theme);

            // --- Partition list ---
            let list_chunks = Layout::default()
                .constraints([Constraint::Min(1)])
                .margin(0)
                .split(chunks[1]);
            draw_partition_list(
                f,
                list_chunks[0],
                &partitions,
                &free_space,
                selected_idx,
                scroll,
                &theme,
                readonly,
            );

            // --- Detail line ---
            let detail = if selected_idx < partitions.len() {
                let p = &partitions[selected_idx];
                let fs_info = p.fs_signature.as_deref().unwrap_or("none");
                Line::from(vec![
                    Span::styled(format!(" Partition {}  ", p.number), theme.accent_style),
                    Span::styled(format!("Type: {}  ", p.ptype), theme.normal_style),
                    Span::styled(format!("Size: {}  ", p.size), theme.normal_style),
                    Span::styled(format!("FS: {}  ", fs_info), theme.muted_style),
                ])
            } else if !free_space.is_empty() {
                let idx = selected_idx - partitions.len();
                let fs = &free_space[idx];
                Line::from(vec![
                    Span::styled(" Free space  ", theme.muted_style),
                    Span::styled(format!("Size: {}  ", fs.size), theme.normal_style),
                ])
            } else {
                Line::from(Span::raw(""))
            };
            f.render_widget(Paragraph::new(detail), chunks[2]);

            // --- Action bar ---
            let actions = if readonly {
                " [Q]uit  [Esc] "
            } else {
                " [N]ew  [D]elete  [R]esize  [T]ype  [F]lags  [W]rite  [Q]uit  [Esc] "
            };
            f.render_widget(
                Paragraph::new(Line::from(Span::styled(actions, theme.muted_style)))
                    .alignment(Alignment::Center),
                chunks[3],
            );

            // --- Confirmation dialog ---
            if let Some(ref confirm) = show_confirm {
                draw_confirm_dialog(f, area, confirm, &theme);
            }
        })?;

        // Handle input
        if show_confirm.is_some() {
            if let Event::Key(key) = event::read()? {
                match key.code {
                    KeyCode::Char('y') | KeyCode::Char('Y') => {
                        let confirm = show_confirm.take().unwrap();
                        match confirm.action {
                            ConfirmAction::DeletePartition(idx) => {
                                // Merge space into free_space
                                let p = &partitions[idx];
                                free_space.push(FreeSpace {
                                    start: p.start.clone(),
                                    end: p.end.clone(),
                                    size: p.size.clone(),
                                });
                                free_space.sort_by(|a, b| a.start.cmp(&b.start));
                                partitions.remove(idx);
                                if selected_idx >= partitions.len() && !partitions.is_empty() {
                                    selected_idx = partitions.len() - 1;
                                }
                            }
                            ConfirmAction::WriteChanges => {
                                // Return the final partition layout
                                let result_json = serde_json::json!({
                                    "partitions": partitions.iter().map(|p| {
                                        serde_json::json!({
                                            "number": p.number,
                                            "start": p.start,
                                            "end": p.end,
                                            "size": p.size,
                                            "type": p.ptype,
                                            "flags": p.flags,
                                        })
                                    }).collect::<Vec<_>>(),
                                    "free_space": free_space.iter().map(|fs| {
                                        serde_json::json!({
                                            "start": fs.start,
                                            "end": fs.end,
                                            "size": fs.size,
                                        })
                                    }).collect::<Vec<_>>(),
                                });
                                break Response {
                                    result: Some(result_json),
                                    cancelled: false,
                                    error: None,
                                };
                            }
                            ConfirmAction::Cancel => {}
                        }
                    }
                    KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
                        show_confirm = None;
                    }
                    _ => {}
                }
            }
            continue;
        }

        match event::read()? {
            Event::Key(key) => match key.code {
                KeyCode::Esc | KeyCode::Char('q') => {
                    break Response { result: None, cancelled: true, error: None };
                }
                KeyCode::Up | KeyCode::Char('k') => {
                    if selected_idx > 0 { selected_idx -= 1; }
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    if selected_idx + 1 < total_items { selected_idx += 1; }
                }
                KeyCode::Char('n') if !readonly => {
                    // New partition: simple prompt for size and type via nested state
                    // For now, create a default partition at end of largest free space
                    if !free_space.is_empty() {
                        // Use largest free space
                        let idx = free_space.iter().enumerate()
                            .max_by(|(_, a), (_, b)| human_size_to_bytes(&a.size).cmp(&human_size_to_bytes(&b.size)))
                            .map(|(i, _)| i)
                            .unwrap_or(0);
                        let fs = &free_space[idx];
                        let new_num = partitions.len() as u32 + 1;
                        partitions.push(Partition {
                            number: new_num,
                            start: fs.start.clone(),
                            end: fs.end.clone(),
                            size: fs.size.clone(),
                            ptype: "Linux filesystem".to_string(),
                            flags: vec![],
                            fs_signature: None,
                        });
                        free_space.remove(idx);
                        selected_idx = partitions.len() - 1;
                    }
                }
                KeyCode::Char('d') if !readonly => {
                    if selected_idx < partitions.len() {
                        let p = &partitions[selected_idx];
                        let msg = if let Some(ref sig) = p.fs_signature {
                            format!("Delete partition {} ({}, {} detected)?\n\nThis cannot be undone.", p.number, p.size, sig)
                        } else {
                            format!("Delete partition {} ({})?\n\nThis cannot be undone.", p.number, p.size)
                        };
                        show_confirm = Some(ConfirmDialog {
                            title: "Delete Partition".to_string(),
                            message: msg,
                            action: ConfirmAction::DeletePartition(selected_idx),
                        });
                    }
                }
                KeyCode::Char('w') if !readonly => {
                    // Build a summary of changes for the confirmation dialog
                    let summary = partitions.iter()
                        .map(|p| format!("  {}  {}  {}", p.number, p.size, p.ptype))
                        .collect::<Vec<_>>()
                        .join("\n");
                    show_confirm = Some(ConfirmDialog {
                        title: "Write Changes".to_string(),
                        message: format!("Apply the following layout to {}?\n\n{}", disk, summary),
                        action: ConfirmAction::WriteChanges,
                    });
                }
                _ => {}
            },
            Event::Mouse(mouse) => match mouse.kind {
                MouseEventKind::ScrollDown => { scroll = (scroll + 1).min(partitions.len().saturating_sub(1) as u16); }
                MouseEventKind::ScrollUp => { scroll = scroll.saturating_sub(1); }
                _ => {}
            },
            _ => {}
        }
    };

    if !is_daemon {
        crossterm::execute!(io::stdout(), crossterm::cursor::Show)?;
        crossterm::execute!(io::stdout(), crossterm::event::DisableMouseCapture)?;
        crossterm::execute!(io::stdout(), crossterm::terminal::LeaveAlternateScreen)?;
        crossterm::terminal::disable_raw_mode()?;
    }
    Ok(result)
}

// --- Helper functions ---

fn human_size_to_bytes(s: &str) -> u64 {
    let s = s.trim().to_uppercase();
    let (num, suffix) = s.split_at(s.find(|c: char| !c.is_ascii_digit() && c != '.').unwrap_or(s.len()));
    let num: f64 = num.parse().unwrap_or(0.0);
    match suffix {
        "B" => num as u64,
        "K" | "KB" => (num * 1024.0) as u64,
        "M" | "MB" => (num * 1024.0 * 1024.0) as u64,
        "G" | "GB" => (num * 1024.0 * 1024.0 * 1024.0) as u64,
        "T" | "TB" => (num * 1024.0 * 1024.0 * 1024.0 * 1024.0) as u64,
        _ => num as u64,
    }
}

fn draw_partition_bar(
    f: &mut Frame,
    area: Rect,
    partitions: &[Partition],
    free_space: &[FreeSpace],
    theme: &Theme,
) {
    let total_width = area.width.saturating_sub(2) as usize;
    if total_width == 0 { return; }

    // Calculate total bytes by summing all partition end bytes (rough)
    // For proportional display, use the largest "end" value as total
    let mut max_end: u64 = 0;
    for p in partitions {
        if let Ok(bytes) = parse_size_to_bytes(&p.end) { max_end = max_end.max(bytes); }
    }
    for fs in free_space {
        if let Ok(bytes) = parse_size_to_bytes(&fs.end) { max_end = max_end.max(bytes); }
    }
    if max_end == 0 { return; }

    let mut spans: Vec<Span> = Vec::new();
    let mut cursor: u64 = 0;

    // Combine partitions and free space sorted by start
    let mut segments: Vec<(&str, u64, u64, Color)> = Vec::new(); // (label, start, end, color)
    for p in partitions {
        if let (Ok(start), Ok(end)) = (parse_size_to_bytes(&p.start), parse_size_to_bytes(&p.end)) {
            segments.push((&p.ptype, start, end, Color::Blue));
        }
    }
    for fs in free_space {
        if let (Ok(start), Ok(end)) = (parse_size_to_bytes(&fs.start), parse_size_to_bytes(&fs.end)) {
            segments.push(("Free", start, end, Color::DarkGray));
        }
    }
    segments.sort_by_key(|s| s.1);

    for (label, start, end, color) in segments {
        if start > cursor {
            // Gap – render as dark
            let gap_width = ((start - cursor) as f64 / max_end as f64 * total_width as f64) as usize;
            if gap_width > 0 {
                spans.push(Span::styled(" ".repeat(gap_width), Style::default().bg(Color::DarkGray)));
            }
        }
        let width = ((end - start) as f64 / max_end as f64 * total_width as f64) as usize;
        if width > 0 {
            spans.push(Span::styled(
                format!("{:^width$}", label, width = width),
                Style::default().bg(color).fg(Color::White),
            ));
        }
        cursor = end;
    }

    let bar = Line::from(spans);
    f.render_widget(Paragraph::new(bar).block(Block::default().borders(Borders::NONE)), area);
}

fn draw_partition_list(
    f: &mut Frame,
    area: Rect,
    partitions: &[Partition],
    free_space: &[FreeSpace],
    selected_idx: usize,
    scroll: u16,
    theme: &Theme,
    readonly: bool,
) {
    let mut lines: Vec<Line> = Vec::new();
    for (i, p) in partitions.iter().enumerate() {
        let is_selected = i == selected_idx;
        let style = if is_selected { theme.selected_style } else { theme.normal_style };
        let cursor = if is_selected { "> " } else { "  " };
        lines.push(Line::from(vec![
            Span::styled(cursor, style),
            Span::styled(
                format!("{:>3}  {:>8}  {:<20}", p.number, p.size, p.ptype),
                style,
            ),
        ]));
    }
    for (i, fs) in free_space.iter().enumerate() {
        let idx = partitions.len() + i;
        let is_selected = idx == selected_idx;
        let style = if is_selected { theme.selected_style } else { theme.muted_style };
        let cursor = if is_selected { "> " } else { "  " };
        lines.push(Line::from(vec![
            Span::styled(cursor, style),
            Span::styled(format!("     {:>8}  Free space", fs.size), style),
        ]));
    }
    if lines.is_empty() {
        lines.push(Line::from(Span::styled("  No partitions", theme.muted_style)));
    }
    f.render_widget(Paragraph::new(lines).scroll((scroll, 0)), area);
}

fn draw_confirm_dialog(f: &mut Frame, area: Rect, confirm: &ConfirmDialog, theme: &Theme) {
    let dialog_area = layout::centered(50, 30, area);
    f.render_widget(Clear, dialog_area);
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Yellow))
        .title(confirm.title.as_str())
        .title_style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD));
    f.render_widget(block, dialog_area);

    let inner = dialog_area.inner(&Margin::new(2, 1));
    f.render_widget(Paragraph::new(confirm.message.as_str()).style(theme.normal_style), inner);

    let hint = Paragraph::new(Line::from(Span::styled("[Y]es  [N]o", theme.accent_style)))
        .alignment(Alignment::Center);
    let hint_area = Rect::new(dialog_area.x, dialog_area.y + dialog_area.height - 2, dialog_area.width, 1);
    f.render_widget(hint, hint_area);
}

fn parse_size_to_bytes(s: &str) -> Result<u64, std::num::ParseFloatError> {
    let s = s.trim();
    if s.ends_with("GiB") {
        Ok((s.trim_end_matches("GiB").trim().parse::<f64>()? * 1024.0 * 1024.0 * 1024.0) as u64)
    } else if s.ends_with("MiB") {
        Ok((s.trim_end_matches("MiB").trim().parse::<f64>()? * 1024.0 * 1024.0) as u64)
    } else if s.ends_with("KiB") {
        Ok((s.trim_end_matches("KiB").trim().parse::<f64>()? * 1024.0) as u64)
    } else if s.ends_with("GB") || s.ends_with('G') {
        Ok((s.trim_end_matches(|c| c == 'G' || c == 'B').trim().parse::<f64>()? * 1000.0 * 1000.0 * 1000.0) as u64)
    } else if s.ends_with("MB") || s.ends_with('M') {
        Ok((s.trim_end_matches(|c| c == 'M' || c == 'B').trim().parse::<f64>()? * 1000.0 * 1000.0) as u64)
    } else {
        Ok(s.parse::<f64>()? as u64)
    }
}

#[derive(Debug)]
struct ConfirmDialog {
    title: String,
    message: String,
    action: ConfirmAction,
}

#[derive(Debug)]
enum ConfirmAction {
    DeletePartition(usize),
    WriteChanges,
    Cancel,
}