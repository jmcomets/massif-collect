use std::cmp;

use tui::{
    widgets::{Widget, BarChart},
    style::{Style, Color, Modifier},
    layout::Rect,
    buffer::Buffer,
};

use crate::{
    Snapshot,
    ui::InputHandler,
};

use termion::event::Key;

pub struct AllocationChartWidget<'a> {
    snapshots: &'a [Snapshot],
    selected: Option<usize>,
}

impl<'a> AllocationChartWidget<'a> {
    pub fn new(snapshots: &'a [Snapshot]) -> Self {
        let selected = if !snapshots.is_empty() { Some(0) } else { None };
        AllocationChartWidget { snapshots, selected }
    }

    pub fn select_first(&mut self) {
        if let Some(selected) = self.selected.as_mut() {
            *selected = 0;
        }
    }

    pub fn select_last(&mut self) {
        if let Some(selected) = self.selected.as_mut() {
            *selected = self.snapshots.len() - 1;
        }
    }

    pub fn select_next(&mut self) {
        self.select_nth_next(1)
    }

    pub fn select_previous(&mut self) {
        self.select_nth_previous(1)
    }

    pub fn select_nth_next(&mut self, n: usize) {
        if let Some(i) = self.selected.as_mut() {
            *i += cmp::min(self.snapshots.len()-1-*i, n);
        }
    }

    pub fn select_nth_previous(&mut self, n: usize) {
        if let Some(i) = self.selected.as_mut() {
            *i -= cmp::min(*i, n);
        }
    }
}

impl<'a> Widget for AllocationChartWidget<'a> {
    fn draw(&mut self, area: Rect, buf: &mut Buffer) {
        let data: Vec<_> = self.snapshots.iter()
            .map(|snapshot| ("", snapshot.tree.sample.bytes as u64))
            .collect();
        BarChart::default()
            // .block(Block::default().title("Snapshots").borders(Borders::ALL))
            .bar_width(10)
            .bar_gap(1)
            .style(Style::default().fg(Color::Red))
            .highlight_style(Style::default().fg(Color::Blue))
            .select(self.selected)
            .label_style(Style::default().fg(Color::White))
            .value_style(Style::default().fg(Color::Black).bg(Color::Red).modifier(Modifier::BOLD))
            .data(&data[..])
            .draw(area, buf)
    }
}

impl<'a> InputHandler for AllocationChartWidget<'a> {
    fn handle_input(&mut self, _area: Rect, input: &Key) {
        match input {
            Key::Down | Key::Char('j') => { self.select_next(); }
            Key::Up | Key::Char('k')   => { self.select_previous(); }

            Key::Right | Key::Char('l') => { self.select_next() }
            Key::Left | Key::Char('h')  => { self.select_previous() }

            Key::Home                  => { self.select_first(); }
            Key::End | Key::Char('G')  => { self.select_last(); }

            // Key::Char('\n') => { self.enter_selected(); }
            // Key::Backspace  => { self.leave_current(); }

            _ => {}
        }

    }
}
