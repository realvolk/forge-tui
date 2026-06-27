use ratatui::layout::Rect;

pub fn centered(width_pct: u16, height_pct: u16, area: Rect) -> Rect {
    let w = (area.width as f32 * width_pct as f32 / 100.0) as u16;
    let h = (area.height as f32 * height_pct as f32 / 100.0) as u16;
    let w = w.min(area.width.saturating_sub(4));
    let h = h.min(area.height.saturating_sub(4));
    let x = area.x + (area.width.saturating_sub(w)) / 2;
    let y = area.y + (area.height.saturating_sub(h)) / 2;
    Rect::new(x, y, w, h)
}