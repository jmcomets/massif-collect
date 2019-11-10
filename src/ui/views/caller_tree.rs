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

macro_rules! color_gradient {
    ($x:expr, $color1:expr, $color2:expr) => {{
        let (r1, g1, b1) = $color1;
        let (r2, g2, b2) = $color2;
        (
            (r1 as f64 + $x * (r2 as f64 - r1 as f64)) as u8,
            (g1 as f64 + $x * (g2 as f64 - g1 as f64)) as u8,
            (b1 as f64 + $x * (b2 as f64 - b1 as f64)) as u8,
        )
    }}
}

impl<'a> Widget for CallerTreeWidget<'a> {
    fn draw(&mut self, area: Rect, buf: &mut Buffer) {
        for (i, (_, nb_callers, allocation, depth, ratio)) in self.0.iter().enumerate().take(area.height as usize) {
            let indent = depth * 2;
            if (indent as u16) < area.right() {
                let (x, y) = (area.left(), area.top() + i as u16);

                // pad the line to include the indent & at least the area's width
                let mut line = String::with_capacity(area.width as usize);
                for _ in 0..indent { line.push(' '); }
                write!(&mut line, "n{}: {} {}", nb_callers, allocation.bytes, allocation.location.to_string()).unwrap();
                for _ in line.len()..area.width as usize { line.push(' '); }

                let mut style = Style::default().fg(Color::Black);

                // highlight the selected item
                if self.0.is_selected(i) {
                    style = style.modifier(Modifier::BOLD);
                }

                // map the allocation ratio to a color
                debug_assert!(ratio <= 1.);
                let (red, green, blue) = color_gradient!(ratio, (0xf5, 0xaf, 0x19), (0xf1, 0x27, 0x11));
                style = style.bg(Color::Rgb(red, green, blue));

                buf.set_string(x, y, line, style);
            }
        }
    }
}
