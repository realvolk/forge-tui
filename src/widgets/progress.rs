use crate::contract::Response;
use crate::layout;
use crate::theme::Theme;
use anyhow::Result;
use crossterm::event::{self, Event, KeyCode};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Layout, Margin},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Gauge, Paragraph, Wrap},
    Terminal, Frame,
};
use std::io::BufRead;
use std::io::BufReader;
use std::process::{Command, Stdio};
use std::sync::mpsc;
use std::thread;
use std::fs;
use std::time::Instant;
use std::io;

pub fn run(title: String, command: Vec<String>, logfile: Option<String>) -> Result<Response> {
    let old_stdout = crate::tty::redirect_stdout()?;

    crossterm::terminal::enable_raw_mode()?;
    crossterm::execute!(io::stdout(), crossterm::terminal::EnterAlternateScreen)?;
    crossterm::execute!(io::stdout(), crossterm::cursor::Hide)?;

    let backend = CrosstermBackend::new(io::stdout());
    let mut terminal = Terminal::new(backend)?;

    let theme = Theme::load();

    let (prog_name, args) = command.split_first()
        .map(|(p, a)| (p.clone(), a.to_vec()))
        .unwrap_or((String::new(), vec![]));

    let mut child = Command::new(&prog_name)
        .args(&args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| anyhow::anyhow!("Failed to start '{}': {}", prog_name, e))?;

    let stdout_stream = child.stdout.take().unwrap();
    let stderr_stream = child.stderr.take().unwrap();

    let (tx, rx) = mpsc::channel::<String>();
    let tx2 = tx.clone();

    thread::spawn(move || {
        let reader = BufReader::new(stdout_stream);
        for line in reader.lines() {
            if let Ok(l) = line {
                if tx.send(l).is_err() { break; }
            }
        }
    });
    thread::spawn(move || {
        let reader = BufReader::new(stderr_stream);
        for line in reader.lines() {
            if let Ok(l) = line {
                let _ = tx2.send(l);
            }
        }
    });

    let mut output = String::new();
    let mut progress: u16 = 0;
    let mut target_progress: u16 = 0;
    let mut current_stage = String::from("Starting...");
    let mut cancelled = false;
    let mut show_raw: bool = false;
    let mut last_activity = Instant::now();

    let stage_markers: Vec<(&str, u16, &str)> = vec![
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
                match key.code {
                    KeyCode::Esc => {
                        let _ = child.kill();
                        cancelled = true;
                        break;
                    }
                    KeyCode::Tab => {
                        show_raw = !show_raw;
                    }
                    _ => {}
                }
            }
        }

        let mut had_output = false;
        while let Ok(line) = rx.try_recv() {
            output.push_str(&line);
            output.push('\n');
            had_output = true;
            last_activity = Instant::now();

            for (marker, pct, label) in &stage_markers {
                if line.contains(marker) {
                    target_progress = target_progress.max(*pct);
                    current_stage = label.to_string();
                    break;
                }
            }

            if let Some(ref log) = logfile {
                let _ = fs::write(log, &output);
            }
        }

        if !had_output && progress < target_progress {
            let elapsed = last_activity.elapsed().as_secs_f32();
            if elapsed > 3.0 {
                let creep = ((elapsed - 3.0) * 2.0) as u16;
                progress = (progress + creep).min(target_progress);
            }
        } else if had_output && progress < target_progress {
            progress = target_progress;
        }

        if let Some(_status) = child.try_wait()? {
            while let Ok(line) = rx.try_recv() {
                output.push_str(&line);
                output.push('\n');
            }
            progress = 100;
            target_progress = 100;
            current_stage = String::from("Complete");
            terminal.draw(|f: &mut Frame| {
                draw_progress(f, &title, &theme, &output, &current_stage, progress, show_raw);
            })?;
            thread::sleep(std::time::Duration::from_millis(800));
            break;
        }

        terminal.draw(|f: &mut Frame| {
            draw_progress(f, &title, &theme, &output, &current_stage, progress, show_raw);
        })?;
    }

    crossterm::execute!(io::stdout(), crossterm::cursor::Show)?;
    crossterm::execute!(io::stdout(), crossterm::terminal::LeaveAlternateScreen)?;
    crossterm::terminal::disable_raw_mode()?;

    crate::tty::restore_stdout(old_stdout);

    Ok(Response {
        result: Some(serde_json::Value::String(output)),
        cancelled,
        error: None,
    })
}

fn draw_progress(
    f: &mut Frame,
    title: &str,
    theme: &Theme,
    output: &str,
    current_stage: &str,
    progress: u16,
    show_raw: bool,
) {
    let area = f.size();

    if let Some(ref wm) = theme.watermark_path {
        crate::watermark::render(f, area, wm);
    }

    let box_area = layout::centered(80, 85, area);
    f.render_widget(Clear, box_area);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(theme.border_style)
        .title(title)
        .title_style(theme.title_style);
    f.render_widget(block, box_area);

    let inner = box_area.inner(&Margin::new(2, 1));

    if show_raw {
        let total_lines = output.lines().count() as u16;
        let visible_lines = inner.height.saturating_sub(1);
        let scroll = total_lines.saturating_sub(visible_lines);

        let paragraph = Paragraph::new(output)
            .style(theme.normal_style)
            .wrap(Wrap { trim: false })
            .scroll((scroll, 0));
        f.render_widget(paragraph, inner);

        let hint_y = box_area.y + box_area.height.saturating_sub(1);
        let hint_x = box_area.x + 2;
        if hint_y < area.height {
            f.render_widget(
                Paragraph::new(Line::from(Span::styled(
                    " [Tab] progress view  [Esc] cancel",
                    theme.muted_style,
                ))),
                ratatui::layout::Rect::new(hint_x, hint_y, box_area.width.saturating_sub(4), 1),
            );
        }
    } else {
        let chunks = Layout::default()
            .constraints([
                Constraint::Length(3),
                Constraint::Length(1),
                Constraint::Length(1),
                Constraint::Min(1),
            ])
            .split(inner);

        let gauge = Gauge::default()
            .gauge_style(Style::default().fg(theme.accent_color).add_modifier(Modifier::BOLD))
            .percent(progress)
            .label(format!("{}%", progress));
        f.render_widget(gauge, chunks[0]);

        let stage_line = Line::from(vec![
            Span::styled(" Stage: ", theme.muted_style),
            Span::styled(current_stage, theme.accent_style),
        ]);
        f.render_widget(Paragraph::new(stage_line), chunks[1]);

        let hint = Paragraph::new(" [Tab] raw output  [Esc] cancel")
            .style(theme.muted_style);
        f.render_widget(hint, chunks[2]);

        let recent: String = output
            .lines()
            .rev()
            .take(15)
            .collect::<Vec<&str>>()
            .into_iter()
            .rev()
            .collect::<Vec<&str>>()
            .join("\n");

        f.render_widget(
            Paragraph::new(recent)
                .style(Style::default().fg(ratatui::style::Color::Gray))
                .wrap(Wrap { trim: false }),
            chunks[3],
        );
    }
}