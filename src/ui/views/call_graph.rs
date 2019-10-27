use tui::{
    backend::Backend,
    layout::{Layout, Constraint, Direction},
    style::{Color, Style},
    terminal::Frame,
    widgets::{Widget, SelectableList, Block, Borders},
};

use crate::ui::controllers::{
    CallListController,
    CallGraphController,
};

pub fn render_call_graph<B: Backend>(call_graph: &CallGraphController, f: &mut Frame<B>) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .margin(1)
        .constraints(
            [
            Constraint::Percentage(50),
            Constraint::Percentage(50)
            ].as_ref()
        )
        .split(f.size());

    let (callers, callers_active) = call_graph.caller_list();
    call_list_widget("Callers", callers, callers_active).render(f, chunks[0]);
    let (callees, callees_active) = call_graph.callee_list();
    call_list_widget("Callees", callees, callees_active).render(f, chunks[1]);
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
