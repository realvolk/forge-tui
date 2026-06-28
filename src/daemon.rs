use crate::contract::{Request, Response};
use crate::theme::Theme;
use crate::watermark;
use crate::widgets;
use anyhow::Result;
use ratatui::{
    backend::CrosstermBackend,
    layout::{Alignment, Constraint, Layout},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::Paragraph,
    Terminal, Frame,
};
use std::io::{self, BufRead, BufReader, Write};
use std::os::unix::net::{UnixListener, UnixStream};
use std::sync::mpsc;
use std::thread;

struct DaemonRequest {
    request: Request,
    response_tx: mpsc::Sender<Response>,
}

pub fn run(listener: UnixListener) -> Result<()> {
    let stdout = crate::tty::open()?;
    crossterm::terminal::enable_raw_mode()?;
    crossterm::execute!(io::stdout(), crossterm::terminal::EnterAlternateScreen)?;
    crossterm::execute!(io::stdout(), crossterm::cursor::Hide)?;

    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let theme = Theme::load();

    let (request_tx, request_rx) = mpsc::channel::<DaemonRequest>();
    thread::spawn(move || {
        for stream in listener.incoming() {
            if let Ok(stream) = stream {
                let tx = request_tx.clone();
                thread::spawn(move || handle_client(stream, tx));
            }
        }
    });

    let mut breadcrumb: Option<(String, u16, u16)> = None;
    let footer_text = String::from("Tab: navigate  Enter: select  Esc: back");

    loop {
        terminal.draw(|f: &mut Frame| {
            let area = f.size();
            if let Some(ref wm) = theme.watermark_path { watermark::render(f, area, wm); }
            let chunks = Layout::default().constraints([Constraint::Length(1), Constraint::Min(1), Constraint::Length(1)]).split(area);

            if let Some((ref title, step, total)) = breadcrumb {
                let left = Span::styled(" ArtixForge ", theme.title_style);
                let step_text = if total > 0 { format!("Step {}/{}", step, total) } else { String::new() };
                let mut spans = vec![left, Span::styled(format!("  {}  ", title), theme.accent_style.add_modifier(Modifier::BOLD))];
                if total > 0 { spans.push(Span::raw(" ")); spans.push(Span::styled(step_text, theme.muted_style)); }
                f.render_widget(Paragraph::new(Line::from(spans)).style(Style::default().bg(theme.border_style.fg.unwrap_or(ratatui::style::Color::DarkGray))), chunks[0]);
            } else {
                f.render_widget(Paragraph::new(Line::from(Span::styled(" Ready ", theme.accent_style))), chunks[0]);
            }

            f.render_widget(Paragraph::new("Waiting for command...").style(theme.muted_style).alignment(Alignment::Center), chunks[1]);
            f.render_widget(Paragraph::new(Line::from(Span::styled(format!("  {}  ", footer_text), theme.muted_style))).alignment(Alignment::Center), chunks[2]);
        })?;

        let daemon_req = match request_rx.recv() {
            Ok(req) => req,
            Err(_) => break,
        };

        if matches!(&daemon_req.request, Request::Quit) {
            let _ = daemon_req.response_tx.send(Response { result: None, cancelled: false, error: None });
            break;
        }

        let (title, step, total) = get_breadcrumb(&daemon_req.request);
        breadcrumb = Some((title, step, total));

        let response = match widgets::dispatch(daemon_req.request, Some(&mut terminal)) {
            Ok(resp) => resp,
            Err(e) => Response { result: None, cancelled: true, error: Some(format!("{}", e)) },
        };

        let _ = daemon_req.response_tx.send(response);
    }

    crossterm::execute!(io::stdout(), crossterm::cursor::Show)?;
    crossterm::execute!(io::stdout(), crossterm::terminal::LeaveAlternateScreen)?;
    crossterm::terminal::disable_raw_mode()?;
    Ok(())
}

fn handle_client(mut stream: UnixStream, request_tx: mpsc::Sender<DaemonRequest>) {
    let mut reader = BufReader::new(stream.try_clone().unwrap());
    let mut line = String::new();
    if reader.read_line(&mut line).is_ok() {
        let line = line.trim().to_string();
        if let Ok(request) = serde_json::from_str::<Request>(&line) {
            let (response_tx, response_rx) = mpsc::channel();
            if request_tx.send(DaemonRequest { request, response_tx }).is_ok() {
                if let Ok(response) = response_rx.recv() {
                    let json = serde_json::to_string(&response).unwrap_or_default();
                    let _ = stream.write_all(json.as_bytes());
                    let _ = stream.write_all(b"\n");
                    let _ = stream.flush();
                }
            }
        }
    }
}

fn get_breadcrumb(request: &Request) -> (String, u16, u16) {
    (String::new(), request.step(), request.total())
}