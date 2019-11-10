use std::fmt::Write;

use tui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Modifier, Style},
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
        for (i, (id, nb_callers, allocation, depth, ratio)) in self.0.iter().enumerate().take(area.height as usize) {
            let indent = depth * 2;
            if (indent as u16) < area.right() {
                let (x, y) = (area.left(), area.top() + i as u16);

                // pad the line to include the indent & at least the area's width
                let mut line = String::with_capacity(area.width as usize);
                for _ in 0..indent { line.push(' '); }
                write!(&mut line, "[{}] {}: {:?}", nb_callers, id, allocation).unwrap();
                for _ in line.len()..area.width as usize { line.push(' '); }

                let mut style = Style::default().fg(Color::Black);

                if self.0.is_selected(i) {
                    style = style.modifier(Modifier::BOLD);
                    // TODO highlight the selected item
                }

                // map the allocation ratio to a color
                debug_assert!(ratio <= 1.);
                let red_level = 255 as u8;
                let other_levels = ((1. - ratio) * 255.) as u8;
                style = style.bg(Color::Rgb(red_level, other_levels, other_levels));

                buf.set_string(x, y, line, style);
            }
        }
    }
}
