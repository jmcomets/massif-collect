use tui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Style},
    widgets::Widget,
};

use crate::ui::controllers::CallerTreeController;

pub struct CallerTreeWidget<'a>(&'a CallerTreeController<'a>);

impl<'a> CallerTreeWidget<'a> {
    pub fn new(controller: &'a CallerTreeController<'a>) -> Self {
        CallerTreeWidget(controller)
    }
}

impl<'a> Widget for CallerTreeWidget<'a> {
    fn draw(&mut self, area: Rect, buf: &mut Buffer) {
        for (y, (id, nb_callers, allocation, depth, ratio)) in self.0.iter().enumerate().take(area.height as usize) {
            let indent = depth as u16 * 2;
            if indent < area.x + area.width {
                let line = format!("[{}] {}: {:?}", nb_callers, id, allocation);
                let mut style = Style::default();
                if self.0.is_selected(y) {
                    style = style.fg(Color::Black);
                }

                let background = if ratio > 0.75 { Color::Rgb(255,   0,   0) }
                            else if ratio > 0.5  { Color::Rgb(255, 165,   0) }
                            else if ratio > 0.25 { Color::Rgb(255, 255,   0) }
                            else                 { Color::Rgb(255, 255, 255) };
                style = style.bg(background);
                buf.set_string(indent, y as u16, line, style);
            }
        }
    }
}
