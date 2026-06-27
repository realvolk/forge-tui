use crate::contract::{ProgressBarConfig, Response};
use crate::layout::centered_rect;
use crate::theme::Theme;
use anyhow::Result;
use crossterm::event::{self, Event, KeyCode};
use crossterm::cursor;
use ratatui::{
    backend::CrosstermBackend,
    layout::{Alignment, Constraint, Direction, Layout},
    style::{Style, Modifier},
    text::{Span, Text},
    widgets::{Block, Borders, Gauge, Paragraph},
    Terminal,
};
use std::io::{self, BufRead, BufReader, Write};
use std::process::{Command, Stdio};
use std::sync::mpsc;
use std::thread;

pub fn run(
    command: String,
    args: Vec<String>,
    progress_bar: Option<ProgressBarConfig>,
    title: String,
) -> Result<Response> {
    let mut stdout = io::stdout();
    crossterm::terminal::enable_raw_mode()?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let theme = Theme::load();

    let stages = progress_bar
        .as_ref()
        .map(|pb| pb.stages.clone())
        .unwrap_or_default();
    let use_progress = !stages.is_empty();

    // Spawn subprocess, capture stdout line by line
    let mut child = Command::new(&command)
        .args(&args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| anyhow::anyhow!("Failed to execute command: {}", e))?;

    let stdout_stream = child.stdout.take().unwrap();
    let stderr_stream = child.stderr.take().unwrap();

    // Channel to send lines to main thread
    let (tx, rx) = mpsc::channel::<String>();
    let tx2 = tx.clone();

    thread::spawn(move || {
        let reader = BufReader::new(stdout_stream);
        for line in reader.lines() {
            if let Ok(l) = line {
                if tx.send(l).is_err() {
                    break;
                }
            }
        }
    });
    thread::spawn(move || {
        let reader = BufReader::new(stderr_stream);
        for line in reader.lines() {
            if let Ok(l) = line {
                let _ = tx2.send(format!("[stderr] {}", l));
            }
        }
    });

    let mut current_stage_index: usize = 0;
    let mut output_log = String::new();
    let mut cancelled = false;

    loop {
        if event::poll(std::time::Duration::from_millis(10))? {
            if let Event::Key(key) = event::read()? {
                if key.code == KeyCode::Esc {
                    let _ = child.kill();
                    cancelled = true;
                    break;
                }
            }
        }

        // Read all available lines from channel
        while let Ok(line) = rx.try_recv() {
            output_log.push_str(&line);
            output_log.push('\n');
            if use_progress && line.starts_with("STAGE:") {
                let stage_name = &line[6..].trim();
                if let Some(pos) = stages.iter().position(|s| s == stage_name) {
                    if pos >= current_stage_index {
                        current_stage_index = pos + 1;
                    }
                }
            }
        }

        if let Some(status) = child.try_wait()? {
            while let Ok(line) = rx.try_recv() {
                output_log.push_str(&line);
                output_log.push('\n');
            }
            let exit_code = status.code().unwrap_or(-1);
            break Ok(Response {
                exit_code: Some(exit_code),
                output: Some(output_log.clone()),
                cancelled,
                ..Default::default()
            });
        }

        terminal.draw(|f| {
            let area = centered_rect(f.area(), 60, 10);
            let block = Block::default()
                .borders(Borders::ALL)
                .title(title.clone())
                .title_style(theme.title_style)
                .border_style(theme.border_style);
            let inner = block.inner(area);

            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints(
                    [
                        Constraint::Length(3),
                        Constraint::Min(1),
                    ]
                    .as_ref(),
                )
                .split(inner);

            if use_progress {
                let progress = if stages.is_empty() {
                    0u16
                } else {
                    (current_stage_index * 100 / stages.len()) as u16
                };
                let label = if current_stage_index < stages.len() {
                    format!("{}", stages[current_stage_index.min(stages.len()-1)])
                } else {
                    "Done".into()
                };
                let gauge = Gauge::default()
                    .block(Block::default())
                    .gauge_style(Style::default().fg(theme.accent_color).bg(theme.background).add_modifier(Modifier::BOLD))
                    .percent(progress as u16)
                    .label(label);
                f.render_widget(gauge, chunks[0]);
            } else {
                let spinner = Paragraph::new("Running...").style(theme.accent_style);
                f.render_widget(spinner, chunks[0]);
            }

            let output_lines: Vec<&str> = output_log.lines().rev().take(5).collect();
            let display = output_lines.join("\n");
            let out_para = Paragraph::new(display).block(Block::default());
            f.render_widget(out_para, chunks[1]);
            f.render_widget(block, area);
        })?;
    }
}