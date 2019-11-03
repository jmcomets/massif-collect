use tui::{
    buffer::Buffer,
    layout::Rect,
    style::Style,
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
        for (y, (caller_id, _, depth)) in self.0.iter().enumerate().take(area.height as usize) {
            let indent = depth as u16 * 2;
            if indent < area.x + area.width {
                let line = format!("Node {:?}", caller_id);
                buf.set_string(indent, y as u16, line, Style::default());
            }
        }
        // Node
        //   Node
        //   Node
    }
}
