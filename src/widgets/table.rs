use crate::contract::{ColumnDef, InfoPanelConfig, TableRow, Response};
use crate::layout::centered_rect;
use crate::theme::Theme;
use anyhow::Result;
use crossterm::event::{self, Event, KeyCode};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::Text,
    widgets::{Block, Borders, Cell, Paragraph, Row, Table, TableState},
    Terminal,
};
use std::io;

pub fn run(
    title: String,
    columns: Vec<ColumnDef>,
    rows: Vec<TableRow>,
    default: Option<String>,
    info_panel: Option<InfoPanelConfig>,
    style_map: Option<serde_json::Value>,
) -> Result<Response> {
    let mut stdout = io::stdout();
    crossterm::terminal::enable_raw_mode()?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let theme = Theme::load();

    let row_ids: Vec<String> = rows.iter().map(|r| r.id.clone()).collect();
    let default_idx = default
        .and_then(|d| row_ids.iter().position(|id| id == &d))
        .unwrap_or(0);
    let mut table_state = TableState::default().with_selected(Some(default_idx));

    let show_info = info_panel.as_ref().map(|ip| ip.enabled).unwrap_or(false);

    let result = loop {
        terminal.draw(|f| {
            let full_area = f.area();
            let (table_area, info_area) = if show_info {
                let chunks = Layout::default()
                    .direction(Direction::Horizontal)
                    .constraints([Constraint::Percentage(60), Constraint::Percentage(40)].as_ref())
                    .split(full_area);
                (chunks[0], Some(chunks[1]))
            } else {
                (full_area, None)
            };

            let table_centered = centered_rect(table_area, table_area.width.saturating_sub(2), table_area.height.saturating_sub(2));
            let block = Block::default()
                .borders(Borders::ALL)
                .title(title.clone())
                .title_style(theme.title_style)
                .border_style(theme.border_style);
            let inner = block.inner(table_centered);

            let header_cells: Vec<Cell> = columns
                .iter()
                .map(|c| Cell::from(Text::raw(c.name.clone())).style(Style::default().add_modifier(Modifier::BOLD)))
                .collect();
            let header = Row::new(header_cells).height(1);

            let rows_widget: Vec<Row> = rows
                .iter()
                .enumerate()
                .map(|(i, row)| {
                    let is_selected = table_state.selected() == Some(i);
                    let cells: Vec<Cell> = columns
                        .iter()
                        .map(|col| {
                            let value = row.cells.get(&col.key)
                                .map(|v| v.as_str().unwrap_or("").to_string())
                                .unwrap_or_default();
                            let mut style = if is_selected {
                                Style::default().fg(theme.accent_color).bg(theme.selected_bg)
                            } else {
                                Style::default().fg(theme.foreground).bg(theme.background)
                            };
                            if let Some(ref map) = style_map {
                                if let Some(color_map) = map.get(&col.key) {
                                    if let Some(color_name) = color_map.get(&value) {
                                        if let Some(color) = parse_color(color_name.as_str().unwrap_or("")) {
                                            style = if is_selected {
                                                style.fg(color)
                                            } else {
                                                style.fg(color)
                                            };
                                        }
                                    }
                                }
                            }
                            Cell::from(Text::raw(value)).style(style)
                        })
                        .collect();
                    Row::new(cells).height(1)
                })
                .collect();

            let table = ratatui::widgets::Table::new(rows_widget, &[])
                .header(header)
                .block(Block::default())
                .highlight_style(Style::default().add_modifier(Modifier::REVERSED))
                .widths(&vec![Constraint::Percentage(100 / columns.len() as u16); columns.len()]);

            f.render_widget(block, table_centered);
            f.render_stateful_widget(table, inner, &mut table_state.clone());

            if let Some(info_rect) = info_area {
                if let Some(selected_idx) = table_state.selected() {
                    if let Some(selected_row) = rows.get(selected_idx) {
                        if let Some(ref meta) = selected_row.meta {
                            let fields = info_panel.as_ref().map(|ip| &ip.fields);
                            let mut text = String::new();
                            if let Some(field_list) = fields {
                                for field in field_list {
                                    if let Some(value) = meta.get(field) {
                                        text.push_str(&format!("{}: {}\n", field, value.as_str().unwrap_or("")));
                                    }
                                }
                            }
                            let info_block = Block::default()
                                .borders(Borders::ALL)
                                .title("Info")
                                .title_style(theme.title_style)
                                .border_style(theme.border_style);
                            let info_centered = centered_rect(info_rect, info_rect.width.saturating_sub(2), info_rect.height.saturating_sub(2));
                            let paragraph = Paragraph::new(text).block(Block::default());
                            f.render_widget(info_block, info_centered);
                            f.render_widget(paragraph, info_block.inner(info_centered));
                        }
                    }
                }
            }
        })?;

        if let Event::Key(key) = event::read()? {
            match key.code {
                KeyCode::Up | KeyCode::Char('k') => {
                    let i = table_state.selected().unwrap_or(0);
                    if i > 0 {
                        table_state.select(Some(i - 1));
                    }
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    let i = table_state.selected().unwrap_or(0);
                    if i < rows.len() - 1 {
                        table_state.select(Some(i + 1));
                    }
                }
                KeyCode::Enter => {
                    let idx = table_state.selected().unwrap_or(default_idx);
                    break Ok(Response {
                        selected: Some(row_ids[idx].clone()),
                        cancelled: false,
                        ..Default::default()
                    });
                }
                KeyCode::Esc => {
                    break Ok(Response {
                        cancelled: true,
                        ..Default::default()
                    });
                }
                _ => {}
            }
        }
    };

    crossterm::terminal::disable_raw_mode()?;
    result
}

fn parse_color(name: &str) -> Option<Color> {
    match name {
        "black" => Some(Color::Black),
        "red" => Some(Color::Red),
        "green" => Some(Color::Green),
        "yellow" => Some(Color::Yellow),
        "blue" => Some(Color::Blue),
        "magenta" => Some(Color::Magenta),
        "cyan" => Some(Color::Cyan),
        "white" => Some(Color::White),
        "gray" | "grey" => Some(Color::Gray),
        "darkgray" => Some(Color::DarkGray),
        "lightred" => Some(Color::LightRed),
        "lightgreen" => Some(Color::LightGreen),
        "lightyellow" => Some(Color::LightYellow),
        "lightblue" => Some(Color::LightBlue),
        "lightmagenta" => Some(Color::LightMagenta),
        "lightcyan" => Some(Color::LightCyan),
        _ => None,
    }
}