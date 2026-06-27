use ratatui::{
    layout::Rect,
    style::{Color, Style},
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};

pub fn render(f: &mut Frame, area: Rect, path: &str) {
    if let Ok(content) = std::fs::read_to_string(path) {
        let lines: Vec<Line> = content
            .lines()
            .map(|l| Line::from(Span::styled(l, Style::default().fg(Color::DarkGray))))
            .collect();

        let wm_height = lines.len() as u16;
        let wm_width = lines
            .iter()
            .map(|l| l.width() as u16)
            .max()
            .unwrap_or(40);

        let wm_area = Rect::new(
            area.x + (area.width.saturating_sub(wm_width)) / 2,
            area.y + (area.height.saturating_sub(wm_height)) / 2,
            wm_width,
            wm_height,
        );

        f.render_widget(Paragraph::new(lines), wm_area);
    }
}