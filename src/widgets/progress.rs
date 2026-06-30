use crate::contract::Response;
use crate::theme::Theme;
use crate::widgets::helpers;
use anyhow::Result;
use crossterm::event::{self, Event, KeyCode};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Gauge, Paragraph, Wrap},
    Terminal, Frame,
};
use std::fs;
use std::fs::File;
use std::io::BufRead;
use std::io::BufReader;
use std::process::{Command, Stdio};
use std::sync::mpsc;
use std::thread;
use std::time::Instant;

pub fn run(
    terminal: Option<&mut Terminal<CrosstermBackend<File>>>,
    title: String,
    command: Vec<String>,
    logfile: Option<String>,
) -> Result<Response> {
    let is_daemon = terminal.is_some();
    let theme = Theme::load();

    let mut owned;
    let term: &mut Terminal<CrosstermBackend<File>> = match terminal {
        Some(t) => t,
        None => {
            owned = helpers::setup_one_shot()?;
            &mut owned
        }
    };

    let (prog, args) = command.split_first().map(|(p, a)| (p.clone(), a.to_vec())).unwrap_or_default();
    let mut child = Command::new(&prog).args(&args).stdout(Stdio::piped()).stderr(Stdio::piped()).spawn()
        .map_err(|e| anyhow::anyhow!("Failed to start '{}': {}", prog, e))?;

    let (tx, rx) = mpsc::channel();
    let tx2 = tx.clone();
    let stdout = child.stdout.take().unwrap();
    let stderr = child.stderr.take().unwrap();
    thread::spawn(move || { for line in BufReader::new(stdout).lines() { if let Ok(l) = line { if tx.send(l).is_err() { break; } } } });
    thread::spawn(move || { for line in BufReader::new(stderr).lines() { if let Ok(l) = line { let _ = tx2.send(l); } } });

    let mut output = String::new();
    let mut progress: u16 = 0;
    let mut target: u16 = 0;
    let mut stage = String::from("Starting...");
    let mut cancelled = false;
    let mut show_raw = false;
    let mut last = Instant::now();

    let markers: Vec<(&str, u16, &str)> = vec![
        ("Preflight dependencies installed.", 5, "Preflight complete"),
        ("Mount setup completed.", 20, "Storage configured"),
        ("Base system installation complete.", 50, "Base system installed"),
        ("All source packages built and installed.", 65, "Packages built"),
        ("Bootloader setup complete.", 78, "Bootloader configured"),
        ("Post-install configuration complete.", 90, "Post-install done"),
        ("Applying final system configuration", 100, "Finalizing"),
    ];

    loop {
        if event::poll(std::time::Duration::from_millis(50))? {
            if let Event::Key(key) = event::read()? {
                if helpers::is_cancel(&Event::Key(key)) {
                    let _ = child.kill(); cancelled = true; break;
                }
                if key.code == KeyCode::Tab { show_raw = !show_raw; }
            }
        }

        let mut had = false;
        while let Ok(line) = rx.try_recv() {
            output.push_str(&line); output.push('\n'); had = true; last = Instant::now();
            for (m, pct, label) in &markers { if line.contains(m) { target = target.max(*pct); stage = label.to_string(); break; } }
            if let Some(ref log) = logfile { let _ = fs::write(log, &output); }
        }
        if !had && progress < target && last.elapsed().as_secs_f32() > 3.0 {
            progress = (progress + 2).min(target);
        } else if had && progress < target { progress = target; }

        if let Some(_) = child.try_wait()? {
            while let Ok(line) = rx.try_recv() { output.push_str(&line); output.push('\n'); }
            progress = 100; stage = "Complete".into();
            term.draw(|f| draw(f, &title, &theme, &output, &stage, progress, show_raw))?;
            thread::sleep(std::time::Duration::from_millis(800));
            break;
        }
        term.draw(|f| draw(f, &title, &theme, &output, &stage, progress, show_raw))?;
    }

    if !is_daemon { helpers::teardown_one_shot()?; }
    Ok(Response { result: Some(serde_json::Value::String(output)), cancelled, error: None })
}

fn draw(f: &mut Frame, title: &str, theme: &Theme, output: &str, stage: &str, pct: u16, raw: bool) {
    let area = f.size();
    if let Some(ref wm) = theme.watermark_path { crate::watermark::render(f, area, wm); }
    let inner = helpers::render_box(f, area, title);

    if raw {
        let tl = output.lines().count() as u16;
        let vl = inner.height.saturating_sub(1);
        f.render_widget(Paragraph::new(output).style(theme.normal_style).wrap(Wrap { trim: false }).scroll((tl.saturating_sub(vl), 0)), inner);
        let hy = inner.y + inner.height.saturating_sub(1);
        if hy < area.height { f.render_widget(helpers::footer("[Tab] progress view  [Esc] cancel  Ctrl+C:quit"), Rect::new(inner.x, hy, inner.width, 1)); }
    } else {
        let chunks = Layout::default().constraints([Constraint::Length(3), Constraint::Length(1), Constraint::Length(1), Constraint::Min(1)]).split(inner);
        f.render_widget(Gauge::default().gauge_style(Style::default().fg(theme.accent_color).add_modifier(Modifier::BOLD)).percent(pct).label(format!("{}%", pct)), chunks[0]);
        f.render_widget(Paragraph::new(Line::from(vec![Span::styled(" Stage: ", theme.muted_style), Span::styled(stage, theme.accent_style)])), chunks[1]);
        f.render_widget(helpers::footer("[Tab] raw output  [Esc] cancel  Ctrl+C:quit"), chunks[2]);
        let recent: String = output.lines().rev().take(15).collect::<Vec<&str>>().into_iter().rev().collect::<Vec<&str>>().join("\n");
        f.render_widget(Paragraph::new(recent).style(Style::default().fg(ratatui::style::Color::Gray)).wrap(Wrap { trim: false }), chunks[3]);
    }
}