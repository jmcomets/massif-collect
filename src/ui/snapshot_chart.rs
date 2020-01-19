use tui::{
    widgets::{Widget, BarChart, Block, Borders},
    style::{Style, Color, Modifier},
    layout::Rect,
    buffer::Buffer,
};

use crate::{
    Snapshot,
};

pub struct SnapshotChartWidget<'a> {
    snapshots: &'a [Snapshot],
}

impl<'a> SnapshotChartWidget<'a> {
    pub fn new(snapshots: &'a [Snapshot]) -> Self {
        SnapshotChartWidget { snapshots }
    }
}

impl<'a> Widget for SnapshotChartWidget<'a> {
    fn draw(&mut self, area: Rect, buf: &mut Buffer) {
        let data: Vec<_> = self.snapshots.iter()
            .map(|snapshot| ("", snapshot.tree.sample.bytes as u64))
            .collect();
        BarChart::default()
            .block(Block::default().title("Snapshots").borders(Borders::ALL))
            .bar_width(10)
            .bar_gap(1)
            .style(Style::default().fg(Color::Red))
            .label_style(Style::default().fg(Color::White))
            .value_style(Style::default().fg(Color::Black).bg(Color::Red).modifier(Modifier::BOLD))
            .data(&data[..])
            .draw(area, buf)
    }
}
