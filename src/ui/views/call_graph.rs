use tui::{
    buffer::Buffer,
    layout::{Layout, Constraint, Direction, Rect},
    style::{Color, Style},
    widgets::{Widget, SelectableList, Block, Borders},
};

use crate::ui::controllers::{
    CallListController,
    CallGraphController,
};

pub struct CallGraphWidget<'a>(&'a CallGraphController<'a>);

impl<'a> CallGraphWidget<'a> {
    pub fn new(controller: &'a CallGraphController<'a>) -> Self {
        CallGraphWidget(controller)
    }
}

impl<'a> Widget for CallGraphWidget<'a> {
    fn draw(&mut self, area: Rect, buf: &mut Buffer) {
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .margin(1)
            .constraints(
                [
                Constraint::Percentage(50),
                Constraint::Percentage(50)
                ].as_ref()
            )
            .split(area);

        let (callers, callers_active) = self.0.caller_list();
        call_list_widget("Callers", callers, callers_active).draw(chunks[0], buf);
        let (callees, callees_active) = self.0.callee_list();
        call_list_widget("Callees", callees, callees_active).draw(chunks[1], buf);
    }
}

fn call_list_widget<'a>(title: &'a str, call_list: &'a CallListController, active: bool) -> SelectableList<'a> {
    let default_style = Style::default().fg(Color::White).bg(Color::Black);

    let highlight_style = {
        if active {
            default_style
                .fg(Color::Black)
                .bg(Color::White)
        } else {
            default_style
                .fg(Color::Gray)
                .bg(Color::Black)
        }
    };

    SelectableList::default()
        .block(Block::default().borders(Borders::ALL).title(title))
        .items(call_list.items())
        .select(call_list.selected_index())
        .style(default_style)
        .highlight_style(highlight_style)
        .highlight_symbol(">")
}
