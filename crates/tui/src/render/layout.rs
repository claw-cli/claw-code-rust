use ratatui::layout::Rect;

pub(super) const COMPOSER_MAX_HEIGHT: u16 = 16;

pub(super) fn centered_content_area(area: Rect) -> Rect {
    area
}

pub(super) fn inner_width(area: Rect) -> u16 {
    area.width.saturating_sub(2).max(1)
}

pub(super) fn inner_height(area: Rect) -> u16 {
    area.height.saturating_sub(2).max(1)
}
