use tui::{
    widgets::Widget,
    style::{Style, Color},
    layout::Rect,
    buffer::Buffer,
};

use crate::{
    Snapshot,
};

pub struct AllocationGraphWidget<'a> {
    snapshots: &'a [Snapshot],
}

impl<'a> AllocationGraphWidget<'a> {
    pub fn new(snapshots: &'a [Snapshot]) -> Self {
        AllocationGraphWidget { snapshots }
    }
}

impl<'a> Widget for AllocationGraphWidget<'a> {
    fn draw(&mut self, area: Rect, buf: &mut Buffer) {
        let max_bytes = self.snapshots.iter()
            .map(|snapshot| snapshot.tree.sample.bytes)
            .max().unwrap_or(0) as f64;
        let nb_snapshots = self.snapshots.len() as f64;

        let width = area.width as f64;
        let height = area.height as f64;
        let style = Style::default().bg(Color::Red);

        let bar_width = width / nb_snapshots;
        for (x, snapshot) in self.snapshots.iter().enumerate() {
            let bar_height = height * snapshot.tree.sample.bytes as f64 / max_bytes;

            let left = (x as f64 * bar_width) as u16;

            for y in 0..bar_height as u16 {
                let top = area.height-1 - y;

                buf.set_stringn(area.left() + left,
                                area.top() + top,
                                " ",
                                bar_width as usize,
                                style);
            }
        }
    }
}
