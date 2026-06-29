use crate::contract::Response;
use crate::layout;
use crate::theme::Theme;
use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, MouseEventKind};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Alignment, Constraint, Layout, Margin, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph},
    Terminal, Frame,
};
use serde_json::Value;
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

#[derive(Debug, Clone, PartialEq)]
enum Mode {
    Main,
    TypePicker(usize),
    FlagPicker(usize),
    ResizeInput(usize),
    NewPartition(usize),      // free space index
    Confirm(ConfirmDialog),
}

#[derive(Debug, Clone, PartialEq)]
struct ConfirmDialog {
    title: String,
    message: String,
    action: ConfirmAction,
}

#[derive(Debug, Clone, PartialEq)]
enum ConfirmAction {
    DeletePartition(usize),
    WriteChanges,
}


fn human_to_bytes(s: &str) -> u64 {
    let s = s.trim().to_uppercase();
    if s.is_empty() { return 0; }
    let (num_str, suffix) = s.split_at(
        s.find(|c: char| !c.is_ascii_digit() && c != '.' && c != '-')
            .unwrap_or(s.len()),
    );
    let num: f64 = num_str.parse().unwrap_or(0.0);
    match suffix {
        "B" => num as u64,
        "K" | "KB" | "KIB" => (num * 1024.0) as u64,
        "M" | "MB" | "MIB" => (num * 1024.0 * 1024.0) as u64,
        "G" | "GB" | "GIB" => (num * 1024.0 * 1024.0 * 1024.0) as u64,
        "T" | "TB" | "TIB" => (num * 1024.0 * 1024.0 * 1024.0 * 1024.0) as u64,
        _ => num as u64,
    }
}

fn bytes_to_human(bytes: u64) -> String {
    if bytes >= 1024 * 1024 * 1024 * 1024 {
        format!("{:.1}TiB", bytes as f64 / (1024.0 * 1024.0 * 1024.0 * 1024.0))
    } else if bytes >= 1024 * 1024 * 1024 {
        format!("{:.1}GiB", bytes as f64 / (1024.0 * 1024.0 * 1024.0))
    } else if bytes >= 1024 * 1024 {
        format!("{:.1}MiB", bytes as f64 / (1024.0 * 1024.0))
    } else if bytes >= 1024 {
        format!("{:.1}KiB", bytes as f64 / 1024.0)
    } else {
        format!("{}B", bytes)
    }
}

fn start_to_bytes(s: &str) -> u64 { human_to_bytes(s) }
fn end_to_bytes(s: &str) -> u64 { human_to_bytes(s) }
fn size_to_bytes(s: &str) -> u64 { human_to_bytes(s) }

fn partition_colors() -> Vec<Color> {
    vec![
        Color::Blue, Color::Cyan, Color::Magenta, Color::Green,
        Color::Red, Color::Yellow, Color::LightBlue, Color::LightCyan,
        Color::LightMagenta, Color::LightGreen, Color::LightRed, Color::LightYellow,
    ]
}

fn color_for_index(idx: usize) -> Color {
    let colors = partition_colors();
    colors[idx % colors.len()]
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
                start: v.get("start").and_then(|s| s.as_str()).unwrap_or("0").to_string(),
                end:   v.get("end").and_then(|s| s.as_str()).unwrap_or("0").to_string(),
                size:  v.get("size").and_then(|s| s.as_str()).unwrap_or("0").to_string(),
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
            let start = v.get("start").and_then(|s| s.as_str()).unwrap_or("0").to_string();
            let end   = v.get("end").and_then(|s| s.as_str()).unwrap_or("0").to_string();
            let size  = v.get("size").and_then(|s| s.as_str()).unwrap_or("0").to_string();

            let end = if end == "0" {
                let start_bytes = human_to_bytes(&start);
                let size_bytes  = human_to_bytes(&size);
                if size_bytes > 0 {
                    bytes_to_human(start_bytes + size_bytes)
                } else {
                    end
                }
            } else {
                end
            };

            free.push(FreeSpace { start, end, size });
        }
    }
    free.sort_by(|a, b| start_to_bytes(&a.start).cmp(&start_to_bytes(&b.start)));
    free
}

fn partition_type_choices() -> Vec<&'static str> {
    vec![
        "EFI System", "BIOS boot", "Linux filesystem", "Linux swap",
        "Linux LVM", "Linux LUKS", "Linux RAID", "Linux /boot",
        "Linux /home", "Linux /var", "Linux /tmp", "Windows data",
        "Windows recovery", "FreeBSD", "FreeBSD swap", "FreeBSD ZFS",
        "FreeBSD UFS", "macOS APFS", "macOS HFS+", "Solaris", "Custom",
    ]
}

fn flag_choices() -> Vec<&'static str> {
    vec!["boot", "esp", "bios_grub", "lvm", "raid"]
}

fn merge_adjacent_free_space(free: &mut Vec<FreeSpace>) {
    free.sort_by(|a, b| start_to_bytes(&a.start).cmp(&start_to_bytes(&b.start)));
    let mut i = 0;
    while i + 1 < free.len() {
        let a_end = end_to_bytes(&free[i].end);
        let b_start = start_to_bytes(&free[i + 1].start);
        if a_end >= b_start {
            let a_start = start_to_bytes(&free[i].start);
            let b_end = end_to_bytes(&free[i + 1].end);
            free[i].end = bytes_to_human(b_end);
            free[i].size = bytes_to_human(b_end - a_start);
            free.remove(i + 1);
        } else {
            i += 1;
        }
    }
}

fn create_partition_from_free_space(
    fs_idx: usize,
    size_str: &str,
    partitions: &mut Vec<Partition>,
    free_space: &mut Vec<FreeSpace>,
) {
    let size_bytes = human_to_bytes(size_str);
    if size_bytes == 0 { return; }

    let fs = &free_space[fs_idx];
    let fs_start_bytes = start_to_bytes(&fs.start);
    let fs_end_bytes = end_to_bytes(&fs.end);
    let fs_size_bytes = fs_end_bytes - fs_start_bytes;

    // Clamp requested size to available free space
    let clamped_size = size_bytes.min(fs_size_bytes);
    if clamped_size == 0 { return; }

    let new_num = partitions.iter().map(|p| p.number).max().unwrap_or(0) + 1;
    let new_start = fs.start.clone();
    let new_end = bytes_to_human(fs_start_bytes + clamped_size);
    let new_size = bytes_to_human(clamped_size);

    partitions.push(Partition {
        number: new_num,
        start: new_start.clone(),
        end: new_end.clone(),
        size: new_size,
        ptype: "Linux filesystem".to_string(),
        flags: vec![],
        fs_signature: None,
    });

    // Adjust free space
    let remaining = fs_size_bytes - clamped_size;
    if remaining > 0 {
        free_space[fs_idx].start = new_end;
        free_space[fs_idx].size = bytes_to_human(remaining);
        free_space[fs_idx].end = bytes_to_human(fs_start_bytes + fs_size_bytes);
    } else {
        free_space.remove(fs_idx);
    }
    merge_adjacent_free_space(free_space);
}

fn apply_resize(
    partitions: &mut Vec<Partition>,
    free_space: &mut Vec<FreeSpace>,
    part_idx: usize,
    new_size_str: &str,
) {
    let new_size_bytes = human_to_bytes(new_size_str);
    if new_size_bytes == 0 { return; }

    let p = &mut partitions[part_idx];
    let old_start = start_to_bytes(&p.start);
    let old_end = end_to_bytes(&p.end);
    let old_size = old_end - old_start;

    if new_size_bytes < old_size {
        let new_end_bytes = old_start + new_size_bytes;
        p.end = bytes_to_human(new_end_bytes);
        p.size = bytes_to_human(new_size_bytes);
        free_space.push(FreeSpace {
            start: bytes_to_human(new_end_bytes),
            end: bytes_to_human(old_end),
            size: bytes_to_human(old_end - new_end_bytes),
        });
    } else if new_size_bytes > old_size {
        let needed = new_size_bytes - old_size;
        if let Some(pos) = free_space.iter().position(|fs| {
            start_to_bytes(&fs.start) <= old_end && end_to_bytes(&fs.end) >= old_end
        }) {
            let fs_start = start_to_bytes(&free_space[pos].start);
            let fs_end = end_to_bytes(&free_space[pos].end);
            let available = fs_end - old_end;
            if available >= needed {
                let new_end_bytes = old_end + needed;
                p.end = bytes_to_human(new_end_bytes);
                p.size = bytes_to_human(new_size_bytes);
                let remainder = fs_end - new_end_bytes;
                if remainder > 0 {
                    free_space[pos].start = bytes_to_human(new_end_bytes);
                    free_space[pos].size = bytes_to_human(remainder);
                } else {
                    free_space.remove(pos);
                }
            }
        }
    }
    merge_adjacent_free_space(free_space);
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
    let mut mode: Mode = Mode::Main;
    let mut type_list_state = ListState::default();
    let mut flag_list_state = ListState::default();
    let mut input_buffer = String::new();

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
        let total_items = partitions.len() + free_space.len();
        if total_items > 0 && mode == Mode::Main {
            selected_idx = selected_idx.min(total_items - 1);
        }

        terminal.draw(|f: &mut Frame| {
            let area = f.size();
            if let Some(ref wm) = theme.watermark_path {
                crate::watermark::render(f, area, wm);
            }

            let box_area = layout::centered(95, 95, area);
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
                    Constraint::Length(5),   // partition bar
                    Constraint::Min(1),      // list / sub‑mode
                    Constraint::Length(2),   // detail
                    Constraint::Length(1),   // action bar
                ])
                .split(inner);

            draw_partition_bar(f, chunks[0], &partitions, &free_space);

            match &mode {
                Mode::Main => {
                    draw_partition_list(f, chunks[1], &partitions, &free_space, selected_idx, scroll, &theme);
                    let detail = build_detail_line(&partitions, &free_space, selected_idx, &theme);
                    f.render_widget(Paragraph::new(detail), chunks[2]);
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
                }
                Mode::TypePicker(part_idx) => {
                    draw_type_picker(f, chunks[1], &partitions, *part_idx, &mut type_list_state, &theme);
                    f.render_widget(Paragraph::new(" j/k:select  Enter:confirm  Esc:cancel "), chunks[3]);
                }
                Mode::FlagPicker(part_idx) => {
                    draw_flag_picker(f, chunks[1], &partitions, *part_idx, &mut flag_list_state, &theme);
                    f.render_widget(Paragraph::new(" j/k:move  Space:toggle  Enter:done  Esc:cancel "), chunks[3]);
                }
                Mode::ResizeInput(part_idx) => {
                    let p = &partitions[*part_idx];
                    let hint = format!(
                        " New size for partition {} (current: {}): {}",
                        p.number, p.size, input_buffer
                    );
                    f.render_widget(Paragraph::new(hint.as_str()).style(theme.accent_style), chunks[1]);
                    f.set_cursor(chunks[1].x + hint.len() as u16, chunks[1].y);
                    f.render_widget(Paragraph::new(" Enter:confirm  Esc:cancel "), chunks[3]);
                }
                Mode::NewPartition(fs_idx) => {
                    let fs = &free_space[*fs_idx];
                    let hint = format!(
                        " New partition size (free: {}): {}",
                        fs.size, input_buffer
                    );
                    f.render_widget(Paragraph::new(hint.as_str()).style(theme.accent_style), chunks[1]);
                    f.set_cursor(chunks[1].x + hint.len() as u16, chunks[1].y);
                    f.render_widget(Paragraph::new(" Enter:confirm  Esc:cancel "), chunks[3]);
                }
                Mode::Confirm(ref confirm) => {
                    draw_confirm_dialog(f, area, confirm, &theme);
                }
            }
        })?;

        match &mode {
            Mode::Confirm(confirm) => {
                let action = confirm.action.clone();
                if let Event::Key(key) = event::read()? {
                    match key.code {
                        KeyCode::Char('y') | KeyCode::Char('Y') => {
                            match action {
                                ConfirmAction::DeletePartition(idx) => {
                                    let p = &partitions[idx];
                                    free_space.push(FreeSpace {
                                        start: p.start.clone(),
                                        end: p.end.clone(),
                                        size: p.size.clone(),
                                    });
                                    partitions.remove(idx);
                                    merge_adjacent_free_space(&mut free_space);
                                    if selected_idx >= partitions.len() && !partitions.is_empty() {
                                        selected_idx = partitions.len() - 1;
                                    }
                                }
                                ConfirmAction::WriteChanges => {
                                    let result_json = serde_json::json!({
                                        "partitions": partitions.iter().map(|p| serde_json::json!({
                                            "number": p.number,
                                            "start": p.start,
                                            "end": p.end,
                                            "size": p.size,
                                            "type": p.ptype,
                                            "flags": p.flags,
                                        })).collect::<Vec<_>>(),
                                        "free_space": free_space.iter().map(|fs| serde_json::json!({
                                            "start": fs.start,
                                            "end": fs.end,
                                            "size": fs.size,
                                        })).collect::<Vec<_>>(),
                                    });
                                    break Response {
                                        result: Some(result_json),
                                        cancelled: false,
                                        error: None,
                                    };
                                }
                            }
                            mode = Mode::Main;
                        }
                        KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
                            mode = Mode::Main;
                        }
                        _ => {}
                    }
                }
                continue;
            }

            Mode::TypePicker(part_idx) => {
                let part_idx = *part_idx;
                if let Event::Key(key) = event::read()? {
                    match key.code {
                        KeyCode::Up | KeyCode::Char('k') => {
                            let i = type_list_state.selected().unwrap_or(0);
                            if i > 0 { type_list_state.select(Some(i - 1)); }
                        }
                        KeyCode::Down | KeyCode::Char('j') => {
                            let types = partition_type_choices();
                            let i = type_list_state.selected().unwrap_or(0);
                            if i + 1 < types.len() { type_list_state.select(Some(i + 1)); }
                        }
                        KeyCode::Enter => {
                            let types = partition_type_choices();
                            let i = type_list_state.selected().unwrap_or(0);
                            partitions[part_idx].ptype = types[i].to_string();
                            mode = Mode::Main;
                        }
                        KeyCode::Esc => { mode = Mode::Main; }
                        _ => {}
                    }
                }
                continue;
            }

            Mode::FlagPicker(part_idx) => {
                let part_idx = *part_idx;
                if let Event::Key(key) = event::read()? {
                    match key.code {
                        KeyCode::Up | KeyCode::Char('k') => {
                            let i = flag_list_state.selected().unwrap_or(0);
                            if i > 0 { flag_list_state.select(Some(i - 1)); }
                        }
                        KeyCode::Down | KeyCode::Char('j') => {
                            let flags = flag_choices();
                            let i = flag_list_state.selected().unwrap_or(0);
                            if i + 1 < flags.len() { flag_list_state.select(Some(i + 1)); }
                        }
                        KeyCode::Char(' ') => {
                            if let Some(sel) = flag_list_state.selected() {
                                let flags = flag_choices();
                                let flag = flags[sel].to_string();
                                let current = &mut partitions[part_idx].flags;
                                if current.contains(&flag) {
                                    current.retain(|f| f != &flag);
                                } else {
                                    current.push(flag);
                                }
                            }
                        }
                        KeyCode::Enter | KeyCode::Esc => { mode = Mode::Main; }
                        _ => {}
                    }
                }
                continue;
            }

            Mode::ResizeInput(part_idx) => {
                let part_idx = *part_idx;
                if let Event::Key(key) = event::read()? {
                    match key.code {
                        KeyCode::Enter => {
                            if !input_buffer.is_empty() {
                                apply_resize(&mut partitions, &mut free_space, part_idx, &input_buffer);
                                input_buffer.clear();
                            }
                            mode = Mode::Main;
                        }
                        KeyCode::Esc => {
                            input_buffer.clear();
                            mode = Mode::Main;
                        }
                        KeyCode::Backspace => { input_buffer.pop(); }
                        KeyCode::Char(c) => { input_buffer.push(c); }
                        _ => {}
                    }
                }
                continue;
            }

            Mode::NewPartition(fs_idx) => {
                let fs_idx = *fs_idx;
                if let Event::Key(key) = event::read()? {
                    match key.code {
                        KeyCode::Enter => {
                            if !input_buffer.is_empty() {
                                create_partition_from_free_space(
                                    fs_idx,
                                    &input_buffer,
                                    &mut partitions,
                                    &mut free_space,
                                );
                                input_buffer.clear();
                                selected_idx = partitions.len() - 1;
                            }
                            mode = Mode::Main;
                        }
                        KeyCode::Esc => {
                            input_buffer.clear();
                            mode = Mode::Main;
                        }
                        KeyCode::Backspace => { input_buffer.pop(); }
                        KeyCode::Char(c) => { input_buffer.push(c); }
                        _ => {}
                    }
                }
                continue;
            }

            Mode::Main => {}
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
                    if selected_idx >= partitions.len() {
                        let fs_idx = selected_idx - partitions.len();
                        input_buffer.clear();
                        mode = Mode::NewPartition(fs_idx);
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
                        mode = Mode::Confirm(ConfirmDialog {
                            title: "Delete Partition".into(),
                            message: msg,
                            action: ConfirmAction::DeletePartition(selected_idx),
                        });
                    }
                }
                KeyCode::Char('t') if !readonly => {
                    if selected_idx < partitions.len() {
                        type_list_state = ListState::default();
                        mode = Mode::TypePicker(selected_idx);
                    }
                }
                KeyCode::Char('f') if !readonly => {
                    if selected_idx < partitions.len() {
                        flag_list_state = ListState::default().with_selected(Some(0));
                        mode = Mode::FlagPicker(selected_idx);
                    }
                }
                KeyCode::Char('r') if !readonly => {
                    if selected_idx < partitions.len() {
                        input_buffer.clear();
                        mode = Mode::ResizeInput(selected_idx);
                    }
                }
                KeyCode::Char('w') if !readonly => {
                    let summary = partitions.iter()
                        .map(|p| format!("  {}  {}  {}", p.number, p.size, p.ptype))
                        .collect::<Vec<_>>()
                        .join("\n");
                    mode = Mode::Confirm(ConfirmDialog {
                        title: "Write Changes".into(),
                        message: format!("Apply the following layout to {}?\n\n{}", disk, summary),
                        action: ConfirmAction::WriteChanges,
                    });
                }
                _ => {}
            },
            Event::Mouse(mouse) => match mouse.kind {
                MouseEventKind::ScrollDown => { scroll = (scroll + 1).min(total_items.saturating_sub(1) as u16); }
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


fn draw_partition_bar(f: &mut Frame, area: Rect, partitions: &[Partition], free_space: &[FreeSpace]) {
    let total_width = area.width.saturating_sub(2) as usize;
    if total_width == 0 { return; }

    if partitions.is_empty() && free_space.len() == 1 {
        let fs = &free_space[0];
        let label = format!("Free: {}", fs.size);
        let span = Span::styled(
            format!("{:^width$}", label, width = total_width),
            Style::default().bg(Color::DarkGray).fg(Color::White),
        );
        f.render_widget(Paragraph::new(Line::from(vec![span])), area);
        return;
    }

    let mut max_end: u64 = 0;
    for p in partitions { max_end = max_end.max(end_to_bytes(&p.end)); }
    for fs in free_space { max_end = max_end.max(end_to_bytes(&fs.end)); }
    if max_end == 0 { return; }

    let mut segments: Vec<(&str, u64, u64, Color)> = Vec::new();
    for (i, p) in partitions.iter().enumerate() {
        let color = color_for_index(i);
        segments.push((&p.ptype, start_to_bytes(&p.start), end_to_bytes(&p.end), color));
    }
    for fs in free_space {
        segments.push(("Free", start_to_bytes(&fs.start), end_to_bytes(&fs.end), Color::DarkGray));
    }
    segments.sort_by_key(|s| s.1);

    let mut spans: Vec<Span> = Vec::new();
    let mut cursor: u64 = 0;
    for (label, start, end, color) in segments {
        if start > cursor {
            let gap = ((start - cursor) as f64 / max_end as f64 * total_width as f64) as usize;
            if gap > 0 {
                spans.push(Span::styled(" ".repeat(gap), Style::default().bg(Color::DarkGray)));
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
    f.render_widget(Paragraph::new(Line::from(spans)), area);
}

fn draw_partition_list(
    f: &mut Frame,
    area: Rect,
    partitions: &[Partition],
    free_space: &[FreeSpace],
    selected_idx: usize,
    scroll: u16,
    theme: &Theme,
) {
    let mut lines: Vec<Line> = Vec::new();
    for (i, p) in partitions.iter().enumerate() {
        let sel = i == selected_idx;
        let style = if sel { theme.selected_style } else { theme.normal_style };
        let cur = if sel { "> " } else { "  " };
        lines.push(Line::from(vec![
            Span::styled(cur, style),
            Span::styled(format!("{:>3}  {:>8}  {:<22}", p.number, p.size, p.ptype), style),
        ]));
    }
    for (i, fs) in free_space.iter().enumerate() {
        let idx = partitions.len() + i;
        let sel = idx == selected_idx;
        let style = if sel { theme.selected_style } else { theme.muted_style };
        let cur = if sel { "> " } else { "  " };
        lines.push(Line::from(vec![
            Span::styled(cur, style),
            Span::styled(format!("     {:>8}  Free space", fs.size), style),
        ]));
    }
    if lines.is_empty() {
        lines.push(Line::from(Span::styled("  No partitions", theme.muted_style)));
    }
    f.render_widget(Paragraph::new(lines).scroll((scroll, 0)), area);
}

fn build_detail_line(
    partitions: &[Partition],
    free_space: &[FreeSpace],
    idx: usize,
    theme: &Theme,
) -> Line<'static> {
    if idx < partitions.len() {
        let p = &partitions[idx];
        let fs = p.fs_signature.as_deref().unwrap_or("none");
        let flags = if p.flags.is_empty() { "none".into() } else { p.flags.join(", ") };
        Line::from(vec![
            Span::styled(format!(" Partition {}  ", p.number), theme.accent_style),
            Span::styled(format!("Type: {}  ", p.ptype), theme.normal_style),
            Span::styled(format!("Size: {}  ", p.size), theme.normal_style),
            Span::styled(format!("FS: {}  ", fs), theme.muted_style),
            Span::styled(format!("Flags: {}  ", flags), theme.muted_style),
        ])
    } else if !free_space.is_empty() {
        let fs = &free_space[idx - partitions.len()];
        Line::from(vec![
            Span::styled(" Free space  ", theme.muted_style),
            Span::styled(format!("Size: {}  ", fs.size), theme.normal_style),
        ])
    } else {
        Line::from(Span::raw(""))
    }
}

fn draw_type_picker(
    f: &mut Frame,
    area: Rect,
    partitions: &[Partition],
    part_idx: usize,
    state: &mut ListState,
    theme: &Theme,
) {
    let types = partition_type_choices();
    let current = &partitions[part_idx].ptype;
    if state.selected().is_none() {
        state.select(Some(types.iter().position(|t| t == current).unwrap_or(0)));
    }
    let items: Vec<ListItem> = types.iter().map(|t| {
        let style = if *t == current { theme.accent_style } else { theme.normal_style };
        ListItem::new(Line::from(Span::styled(t.to_string(), style)))
    }).collect();
    f.render_stateful_widget(
        List::new(items).highlight_style(theme.selected_style).highlight_symbol("> "),
        area, state,
    );
}

fn draw_flag_picker(
    f: &mut Frame,
    area: Rect,
    partitions: &[Partition],
    part_idx: usize,
    state: &mut ListState,
    theme: &Theme,
) {
    let flags = flag_choices();
    let current = &partitions[part_idx].flags;
    let items: Vec<ListItem> = flags.iter().enumerate().map(|(i, f)| {
        let active = current.contains(&f.to_string());
        let mark = if active { "[x]" } else { "[ ]" };
        let style = if Some(i) == state.selected() { theme.selected_style }
                    else if active { theme.accent_style }
                    else { theme.normal_style };
        ListItem::new(Line::from(Span::styled(format!(" {} {}", mark, f), style)))
    }).collect();
    f.render_stateful_widget(
        List::new(items).highlight_style(theme.selected_style),
        area, state,
    );
}

fn draw_confirm_dialog(f: &mut Frame, area: Rect, confirm: &ConfirmDialog, theme: &Theme) {
    let dialog_area = layout::centered(55, 35, area);
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
    f.render_widget(hint, Rect::new(dialog_area.x, dialog_area.y + dialog_area.height - 2, dialog_area.width, 1));
}