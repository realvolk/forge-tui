use crate::contract::Response;
use crate::layout;
use crate::theme::Theme;
use crate::widgets::helpers;
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

#[derive(Debug, Clone)]
struct Partition {
    number: u32, start: String, end: String, size: String, ptype: String,
    flags: Vec<String>, fs_signature: Option<String>,
}
#[derive(Debug, Clone)]
struct FreeSpace { start: String, end: String, size: String }

#[derive(Debug, Clone, PartialEq)]
enum Mode {
    Main, TypePicker(usize), FlagPicker(usize), ResizeInput(usize),
    NewPartition(usize), Confirm(ConfirmDialog),
}
#[derive(Debug, Clone, PartialEq)]
struct ConfirmDialog { title: String, message: String, action: ConfirmAction }
#[derive(Debug, Clone, PartialEq)]
enum ConfirmAction { DeletePartition(usize), WriteChanges }

fn human_to_bytes(s: &str) -> u64 {
    let s = s.trim().to_uppercase();
    if s.is_empty() { return 0; }
    let (num_str, suffix) = s.split_at(s.find(|c: char| !c.is_ascii_digit() && c != '.' && c != '-').unwrap_or(s.len()));
    let num: f64 = num_str.parse().unwrap_or(0.0);
    match suffix {
        "B" => num as u64,
        "K"|"KB"|"KIB" => (num * 1024.0) as u64,
        "M"|"MB"|"MIB" => (num * 1024.0 * 1024.0) as u64,
        "G"|"GB"|"GIB" => (num * 1024.0 * 1024.0 * 1024.0) as u64,
        "T"|"TB"|"TIB" => (num * 1024.0 * 1024.0 * 1024.0 * 1024.0) as u64,
        _ => num as u64,
    }
}
fn bytes_to_human(bytes: u64) -> String {
    if bytes >= 1024*1024*1024*1024 { format!("{:.1}TiB", bytes as f64 / (1024.0*1024.0*1024.0*1024.0)) }
    else if bytes >= 1024*1024*1024 { format!("{:.1}GiB", bytes as f64 / (1024.0*1024.0*1024.0)) }
    else if bytes >= 1024*1024 { format!("{:.1}MiB", bytes as f64 / (1024.0*1024.0)) }
    else if bytes >= 1024 { format!("{:.1}KiB", bytes as f64 / 1024.0) }
    else { format!("{}B", bytes) }
}
fn start_to_bytes(s: &str) -> u64 { human_to_bytes(s) }
fn end_to_bytes(s: &str) -> u64 { human_to_bytes(s) }
fn size_to_bytes(s: &str) -> u64 { human_to_bytes(s) }

fn partition_colors() -> Vec<Color> {
    vec![Color::Blue, Color::Cyan, Color::Magenta, Color::Green, Color::Red, Color::Yellow,
         Color::LightBlue, Color::LightCyan, Color::LightMagenta, Color::LightGreen, Color::LightRed, Color::LightYellow]
}

fn parse_partitions(json: &Value) -> Vec<Partition> {
    let mut parts = Vec::new();
    if let Some(arr) = json.as_array() {
        for v in arr {
            let flags: Vec<String> = v.get("flags").and_then(|f| f.as_array())
                .map(|a| a.iter().filter_map(|s| s.as_str().map(String::from)).collect()).unwrap_or_default();
            parts.push(Partition {
                number: v.get("number").and_then(|n| n.as_u64()).unwrap_or(0) as u32,
                start: v.get("start").and_then(|s| s.as_str()).unwrap_or("0").to_string(),
                end:   v.get("end").and_then(|s| s.as_str()).unwrap_or("0").to_string(),
                size:  v.get("size").and_then(|s| s.as_str()).unwrap_or("0").to_string(),
                ptype: v.get("type").and_then(|s| s.as_str()).unwrap_or("").to_string(),
                flags, fs_signature: v.get("fs_signature").and_then(|s| s.as_str()).map(String::from),
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
                let sb = human_to_bytes(&start); let sz = human_to_bytes(&size);
                if sz > 0 { bytes_to_human(sb + sz) } else { end }
            } else { end };
            free.push(FreeSpace { start, end, size });
        }
    }
    free.sort_by(|a, b| start_to_bytes(&a.start).cmp(&start_to_bytes(&b.start)));
    free
}

fn partition_type_choices() -> Vec<&'static str> {
    vec!["EFI System","BIOS boot","Linux filesystem","Linux swap","Linux LVM","Linux LUKS","Linux RAID",
         "Linux /boot","Linux /home","Linux /var","Linux /tmp","Windows data","Windows recovery",
         "FreeBSD","FreeBSD swap","FreeBSD ZFS","FreeBSD UFS","macOS APFS","macOS HFS+","Solaris","Custom"]
}
fn flag_choices() -> Vec<&'static str> { vec!["boot","esp","bios_grub","lvm","raid"] }

fn merge_adjacent_free_space(free: &mut Vec<FreeSpace>) {
    free.sort_by(|a, b| start_to_bytes(&a.start).cmp(&start_to_bytes(&b.start)));
    let mut i = 0;
    while i + 1 < free.len() {
        let ae = end_to_bytes(&free[i].end); let bs = start_to_bytes(&free[i+1].start);
        if ae >= bs {
            let a_start = start_to_bytes(&free[i].start); let b_end = end_to_bytes(&free[i+1].end);
            free[i].end = bytes_to_human(b_end); free[i].size = bytes_to_human(b_end - a_start);
            free.remove(i+1);
        } else { i += 1; }
    }
}

fn create_partition_from_free(fs_idx: usize, size_str: &str, parts: &mut Vec<Partition>, free: &mut Vec<FreeSpace>) {
    let size_bytes = human_to_bytes(size_str); if size_bytes == 0 { return; }
    let fs = &free[fs_idx];
    let fs_start = start_to_bytes(&fs.start); let fs_end = end_to_bytes(&fs.end);
    let clamped = size_bytes.min(fs_end - fs_start); if clamped == 0 { return; }
    let num = parts.iter().map(|p| p.number).max().unwrap_or(0) + 1;
    parts.push(Partition { number: num, start: fs.start.clone(), end: bytes_to_human(fs_start + clamped),
        size: bytes_to_human(clamped), ptype: "Linux filesystem".into(), flags: vec![], fs_signature: None });
    let rem = fs_end - fs_start - clamped;
    if rem > 0 { free[fs_idx].start = bytes_to_human(fs_start + clamped); free[fs_idx].size = bytes_to_human(rem); }
    else { free.remove(fs_idx); }
    merge_adjacent_free_space(free);
}

fn apply_resize(parts: &mut Vec<Partition>, free: &mut Vec<FreeSpace>, idx: usize, new_size: &str) {
    let new_bytes = human_to_bytes(new_size); if new_bytes == 0 { return; }
    let p = &mut parts[idx];
    let old_start = start_to_bytes(&p.start); let old_end = end_to_bytes(&p.end); let old_size = old_end - old_start;
    if new_bytes < old_size {
        let new_end = old_start + new_bytes; p.end = bytes_to_human(new_end); p.size = bytes_to_human(new_bytes);
        free.push(FreeSpace { start: bytes_to_human(new_end), end: bytes_to_human(old_end), size: bytes_to_human(old_end - new_end) });
    } else if new_bytes > old_size {
        let needed = new_bytes - old_size;
        if let Some(pos) = free.iter().position(|fs| start_to_bytes(&fs.start) <= old_end && end_to_bytes(&fs.end) >= old_end) {
            let fs_end = end_to_bytes(&free[pos].end); let avail = fs_end - old_end;
            if avail >= needed {
                p.end = bytes_to_human(old_end + needed); p.size = bytes_to_human(new_bytes);
                let rem = fs_end - old_end - needed;
                if rem > 0 { free[pos].start = bytes_to_human(old_end + needed); free[pos].size = bytes_to_human(rem); }
                else { free.remove(pos); }
            }
        }
    }
    merge_adjacent_free_space(free);
}


pub fn run(
    terminal: Option<&mut Terminal<CrosstermBackend<File>>>,
    title: String, disk: String, partitions_json: Value,
    free_space_json: Option<Value>, readonly: Option<bool>,
) -> Result<Response> {
    let is_daemon = terminal.is_some();
    let readonly = readonly.unwrap_or(false);
    let theme = Theme::load();
    let mut partitions = parse_partitions(&partitions_json);
    let mut free_space = parse_free_space(&free_space_json.unwrap_or(Value::Null));
    let mut sel: usize = 0; let mut scroll: u16 = 0; let mut mode = Mode::Main;
    let mut type_st = ListState::default(); let mut flag_st = ListState::default(); let mut input = String::new();

    let mut owned;
    let term: &mut Terminal<CrosstermBackend<File>> = match terminal {
        Some(t) => t,
        None => {
            helpers::enable_mouse()?;
            owned = helpers::setup_one_shot()?;
            &mut owned
        }
    };

    let result = loop {
        let total = partitions.len() + free_space.len();
        if total > 0 && mode == Mode::Main { sel = sel.min(total - 1); }

        term.draw(|f: &mut Frame| {
            let area = f.size();
            if let Some(ref wm) = theme.watermark_path { crate::watermark::render(f, area, wm); }
            let box_area = layout::centered(95, 95, area);
            f.render_widget(Clear, box_area);
            let block = Block::default().borders(Borders::ALL).border_style(theme.border_style)
                .title(format!("{} ({})", title, disk)).title_style(theme.title_style);
            f.render_widget(block, box_area);
            let inner = box_area.inner(&Margin::new(1, 1));
            let chunks = Layout::default().constraints([Constraint::Length(5), Constraint::Min(1), Constraint::Length(2), Constraint::Length(1)]).split(inner);

            draw_partition_bar(f, chunks[0], &partitions, &free_space);

            match &mode {
                Mode::Main => {
                    draw_partition_list(f, chunks[1], &partitions, &free_space, sel, scroll, &theme);
                    f.render_widget(Paragraph::new(build_detail(&partitions, &free_space, sel, &theme)), chunks[2]);
                    let acts = if readonly { " [Q]uit  [Esc] " } else { " [N]ew  [D]elete  [R]esize  [T]ype  [F]lags  [W]rite  [Q]uit  [Esc]  Ctrl+C:quit" };
                    f.render_widget(Paragraph::new(Line::from(Span::styled(acts, theme.muted_style))).alignment(Alignment::Center), chunks[3]);
                }
                Mode::TypePicker(pi) => {
                    draw_type_picker(f, chunks[1], &partitions, *pi, &mut type_st, &theme);
                    f.render_widget(Paragraph::new(" j/k:select  Enter:confirm  Esc:cancel "), chunks[3]);
                }
                Mode::FlagPicker(pi) => {
                    draw_flag_picker(f, chunks[1], &partitions, *pi, &mut flag_st, &theme);
                    f.render_widget(Paragraph::new(" j/k:move  Space:toggle  Enter:done  Esc:cancel "), chunks[3]);
                }
                Mode::ResizeInput(pi) => {
                    let p = &partitions[*pi];
                    let hint = format!(" New size for partition {} (current: {}): {}", p.number, p.size, input);
                    f.render_widget(Paragraph::new(hint.as_str()).style(theme.accent_style), chunks[1]);
                    f.set_cursor(chunks[1].x + hint.len() as u16, chunks[1].y);
                    f.render_widget(Paragraph::new(" Enter:confirm  Esc:cancel "), chunks[3]);
                }
                Mode::NewPartition(fi) => {
                    let fs = &free_space[*fi];
                    let hint = format!(" New partition size (free: {}): {}", fs.size, input);
                    f.render_widget(Paragraph::new(hint.as_str()).style(theme.accent_style), chunks[1]);
                    f.set_cursor(chunks[1].x + hint.len() as u16, chunks[1].y);
                    f.render_widget(Paragraph::new(" Enter:confirm  Esc:cancel "), chunks[3]);
                }
                Mode::Confirm(ref c) => draw_confirm_dialog(f, area, c, &theme),
            }
        })?;

        match &mode {
            Mode::Confirm(c) => {
                let action = c.action.clone();
                if let Event::Key(key) = event::read()? {
                    match key.code {
                        KeyCode::Char('y')|KeyCode::Char('Y') => {
                            match action {
                                ConfirmAction::DeletePartition(idx) => {
                                    let p = &partitions[idx];
                                    free_space.push(FreeSpace { start: p.start.clone(), end: p.end.clone(), size: p.size.clone() });
                                    partitions.remove(idx); merge_adjacent_free_space(&mut free_space);
                                    if sel >= partitions.len() && !partitions.is_empty() { sel = partitions.len() - 1; }
                                }
                                ConfirmAction::WriteChanges => {
                                    let result_json = serde_json::json!({
                                        "partitions": partitions.iter().map(|p| serde_json::json!({
                                            "number": p.number,"start": p.start,"end": p.end,"size": p.size,"type": p.ptype,"flags": p.flags
                                        })).collect::<Vec<_>>(),
                                        "free_space": free_space.iter().map(|fs| serde_json::json!({
                                            "start": fs.start,"end": fs.end,"size": fs.size
                                        })).collect::<Vec<_>>(),
                                    });
                                    break Response { result: Some(result_json), cancelled: false, error: None };
                                }
                            }
                            mode = Mode::Main;
                        }
                        KeyCode::Char('n')|KeyCode::Char('N')|KeyCode::Esc => { mode = Mode::Main; }
                        _ => {}
                    }
                }
                continue;
            }
            Mode::TypePicker(pi) => {
                let pi = *pi;
                if let Event::Key(key) = event::read()? {
                    match key.code {
                        KeyCode::Up|KeyCode::Char('k') => { let i = type_st.selected().unwrap_or(0); if i > 0 { type_st.select(Some(i-1)); } }
                        KeyCode::Down|KeyCode::Char('j') => { let types = partition_type_choices(); let i = type_st.selected().unwrap_or(0); if i+1 < types.len() { type_st.select(Some(i+1)); } }
                        KeyCode::Enter => { let types = partition_type_choices(); partitions[pi].ptype = types[type_st.selected().unwrap_or(0)].to_string(); mode = Mode::Main; }
                        KeyCode::Esc => { mode = Mode::Main; }
                        _ => {}
                    }
                }
                continue;
            }
            Mode::FlagPicker(pi) => {
                let pi = *pi;
                if let Event::Key(key) = event::read()? {
                    match key.code {
                        KeyCode::Up|KeyCode::Char('k') => { let i = flag_st.selected().unwrap_or(0); if i > 0 { flag_st.select(Some(i-1)); } }
                        KeyCode::Down|KeyCode::Char('j') => { let flags = flag_choices(); let i = flag_st.selected().unwrap_or(0); if i+1 < flags.len() { flag_st.select(Some(i+1)); } }
                        KeyCode::Char(' ') => {
                            if let Some(sel) = flag_st.selected() {
                                let flag = flag_choices()[sel].to_string();
                                let current = &mut partitions[pi].flags;
                                if current.contains(&flag) { current.retain(|f| f != &flag); } else { current.push(flag); }
                            }
                        }
                        KeyCode::Enter|KeyCode::Esc => { mode = Mode::Main; }
                        _ => {}
                    }
                }
                continue;
            }
            Mode::ResizeInput(pi) => {
                let pi = *pi;
                if let Event::Key(key) = event::read()? {
                    match key.code {
                        KeyCode::Enter => { if !input.is_empty() { apply_resize(&mut partitions, &mut free_space, pi, &input); input.clear(); } mode = Mode::Main; }
                        KeyCode::Esc => { input.clear(); mode = Mode::Main; }
                        KeyCode::Backspace => { input.pop(); }
                        KeyCode::Char(c) => { input.push(c); }
                        _ => {}
                    }
                }
                continue;
            }
            Mode::NewPartition(fi) => {
                let fi = *fi;
                if let Event::Key(key) = event::read()? {
                    match key.code {
                        KeyCode::Enter => { if !input.is_empty() { create_partition_from_free(fi, &input, &mut partitions, &mut free_space); input.clear(); sel = partitions.len()-1; } mode = Mode::Main; }
                        KeyCode::Esc => { input.clear(); mode = Mode::Main; }
                        KeyCode::Backspace => { input.pop(); }
                        KeyCode::Char(c) => { input.push(c); }
                        _ => {}
                    }
                }
                continue;
            }
            Mode::Main => {}
        }

        match event::read()? {
            Event::Key(key) => {
                if helpers::is_cancel(&Event::Key(key)) { break Response { result: None, cancelled: true, error: None }; }
                match key.code {
                    KeyCode::Esc|KeyCode::Char('q') => break Response { result: None, cancelled: true, error: None },
                    KeyCode::Up|KeyCode::Char('k') => { if sel > 0 { sel -= 1; } }
                    KeyCode::Down|KeyCode::Char('j') => { if sel+1 < total { sel += 1; } }
                    KeyCode::Char('n') if !readonly => {
                        if sel >= partitions.len() { let fi = sel - partitions.len(); input.clear(); mode = Mode::NewPartition(fi); }
                    }
                    KeyCode::Char('d') if !readonly => {
                        if sel < partitions.len() {
                            let p = &partitions[sel];
                            let msg = if let Some(ref sig) = p.fs_signature {
                                format!("Delete partition {} ({}, {} detected)?\n\nThis cannot be undone.", p.number, p.size, sig)
                            } else { format!("Delete partition {} ({})?\n\nThis cannot be undone.", p.number, p.size) };
                            mode = Mode::Confirm(ConfirmDialog { title: "Delete Partition".into(), message: msg, action: ConfirmAction::DeletePartition(sel) });
                        }
                    }
                    KeyCode::Char('t') if !readonly => { if sel < partitions.len() { type_st = ListState::default(); mode = Mode::TypePicker(sel); } }
                    KeyCode::Char('f') if !readonly => { if sel < partitions.len() { flag_st = ListState::default().with_selected(Some(0)); mode = Mode::FlagPicker(sel); } }
                    KeyCode::Char('r') if !readonly => { if sel < partitions.len() { input.clear(); mode = Mode::ResizeInput(sel); } }
                    KeyCode::Char('w') if !readonly => {
                        let summary = partitions.iter().map(|p| format!("  {}  {}  {}", p.number, p.size, p.ptype)).collect::<Vec<_>>().join("\n");
                        mode = Mode::Confirm(ConfirmDialog { title: "Write Changes".into(), message: format!("Apply to {}?\n\n{}", disk, summary), action: ConfirmAction::WriteChanges });
                    }
                    _ => {}
                }
            }
            Event::Mouse(m) => match m.kind {
                MouseEventKind::ScrollDown => { scroll = (scroll+1).min(total.saturating_sub(1) as u16); }
                MouseEventKind::ScrollUp => { scroll = scroll.saturating_sub(1); }
                _ => {}
            },
            _ => {}
        }
    };

    if !is_daemon { helpers::disable_mouse()?; helpers::teardown_one_shot()?; }
    Ok(result)
}

// ── drawing helpers (unchanged) ───────────────────────────────────────
fn draw_partition_bar(f: &mut Frame, area: Rect, parts: &[Partition], free: &[FreeSpace]) {
    let tw = area.width.saturating_sub(2) as usize; if tw == 0 { return; }
    if parts.is_empty() && free.len() == 1 {
        let fs = &free[0];
        let label = format!("Free: {}", fs.size);
        f.render_widget(Paragraph::new(Line::from(vec![Span::styled(format!("{:^w$}", label, w=tw), Style::default().bg(Color::DarkGray).fg(Color::White))])), area);
        return;
    }
    let mut max_end: u64 = 0;
    for p in parts { max_end = max_end.max(end_to_bytes(&p.end)); }
    for fs in free { max_end = max_end.max(end_to_bytes(&fs.end)); }
    if max_end == 0 { return; }
    let mut segs: Vec<(&str, u64, u64, Color)> = Vec::new();
    for (i, p) in parts.iter().enumerate() {
        let colors = partition_colors();
        segs.push((&p.ptype, start_to_bytes(&p.start), end_to_bytes(&p.end), colors[i % colors.len()]));
    }
    for fs in free { segs.push(("Free", start_to_bytes(&fs.start), end_to_bytes(&fs.end), Color::DarkGray)); }
    segs.sort_by_key(|s| s.1);
    let mut spans: Vec<Span> = Vec::new(); let mut cur: u64 = 0;
    for (label, s, e, color) in segs {
        if s > cur { let gap = ((s-cur) as f64 / max_end as f64 * tw as f64) as usize; if gap > 0 { spans.push(Span::styled(" ".repeat(gap), Style::default().bg(Color::DarkGray))); } }
        let w = ((e-s) as f64 / max_end as f64 * tw as f64) as usize;
        if w > 0 { spans.push(Span::styled(format!("{:^w$}", label, w=w), Style::default().bg(color).fg(Color::White))); }
        cur = e;
    }
    f.render_widget(Paragraph::new(Line::from(spans)), area);
}

fn draw_partition_list(f: &mut Frame, area: Rect, parts: &[Partition], free: &[FreeSpace], sel: usize, scroll: u16, theme: &Theme) {
    let mut lines: Vec<Line> = Vec::new();
    for (i, p) in parts.iter().enumerate() {
        let is = i == sel; let style = if is { theme.selected_style } else { theme.normal_style };
        lines.push(Line::from(vec![Span::styled(if is { "> " } else { "  " }, style), Span::styled(format!("{:>3}  {:>8}  {:<22}", p.number, p.size, p.ptype), style)]));
    }
    for (i, fs) in free.iter().enumerate() {
        let idx = parts.len() + i; let is = idx == sel;
        let style = if is { theme.selected_style } else { theme.muted_style };
        lines.push(Line::from(vec![Span::styled(if is { "> " } else { "  " }, style), Span::styled(format!("     {:>8}  Free space", fs.size), style)]));
    }
    if lines.is_empty() { lines.push(Line::from(Span::styled("  No partitions", theme.muted_style))); }
    f.render_widget(Paragraph::new(lines).scroll((scroll, 0)), area);
}

fn build_detail(parts: &[Partition], free: &[FreeSpace], idx: usize, theme: &Theme) -> Line<'static> {
    if idx < parts.len() {
        let p = &parts[idx];
        let fs = p.fs_signature.as_deref().unwrap_or("none");
        let flag_str = if p.flags.is_empty() {
            "none".to_string()
        } else {
            p.flags.join(", ")
        };
        Line::from(vec![
            Span::styled(format!(" Partition {}  ", p.number), theme.accent_style),
            Span::styled(format!("Type: {}  ", p.ptype), theme.normal_style),
            Span::styled(format!("Size: {}  ", p.size), theme.normal_style),
            Span::styled(format!("FS: {}  ", fs), theme.muted_style),
            Span::styled(format!("Flags: {}  ", flag_str), theme.muted_style),
        ])
    } else if !free.is_empty() {
        let fs = &free[idx - parts.len()];
        Line::from(vec![
            Span::styled(" Free space  ", theme.muted_style),
            Span::styled(format!("Size: {}  ", fs.size), theme.normal_style),
        ])
    } else {
        Line::from(Span::raw(""))
    }
}

fn draw_type_picker(f: &mut Frame, area: Rect, parts: &[Partition], pi: usize, st: &mut ListState, theme: &Theme) {
    let types = partition_type_choices(); let cur = &parts[pi].ptype;
    if st.selected().is_none() { st.select(Some(types.iter().position(|t| t == cur).unwrap_or(0))); }
    let items: Vec<ListItem> = types.iter().map(|t| {
        ListItem::new(Line::from(Span::styled(t.to_string(), if *t == cur { theme.accent_style } else { theme.normal_style })))
    }).collect();
    f.render_stateful_widget(List::new(items).highlight_style(theme.selected_style).highlight_symbol("> "), area, st);
}
fn draw_flag_picker(f: &mut Frame, area: Rect, parts: &[Partition], pi: usize, st: &mut ListState, theme: &Theme) {
    let flags = flag_choices(); let cur = &parts[pi].flags;
    let items: Vec<ListItem> = flags.iter().enumerate().map(|(i, f)| {
        let act = cur.contains(&f.to_string());
        let style = if Some(i) == st.selected() { theme.selected_style } else if act { theme.accent_style } else { theme.normal_style };
        ListItem::new(Line::from(Span::styled(format!(" {} {}", if act { "[x]" } else { "[ ]" }, f), style)))
    }).collect();
    f.render_stateful_widget(List::new(items).highlight_style(theme.selected_style), area, st);
}
fn draw_confirm_dialog(f: &mut Frame, area: Rect, c: &ConfirmDialog, theme: &Theme) {
    let da = layout::centered(55, 35, area);
    f.render_widget(Clear, da);
    let block = Block::default().borders(Borders::ALL).border_style(Style::default().fg(Color::Yellow))
        .title(c.title.as_str()).title_style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD));
    f.render_widget(block, da);
    let inner = da.inner(&Margin::new(2, 1));
    f.render_widget(Paragraph::new(c.message.as_str()).style(theme.normal_style), inner);
    f.render_widget(Paragraph::new(Line::from(Span::styled("[Y]es  [N]o", theme.accent_style))).alignment(Alignment::Center),
        Rect::new(da.x, da.y + da.height - 2, da.width, 1));
}