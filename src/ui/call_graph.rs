use std::{
    ops::{Deref, DerefMut},
    cmp,
};

use petgraph::prelude::*;

use tui::{
    buffer::Buffer,
    layout::{Layout, Constraint, Direction, Rect},
    style::{Color, Style},
    widgets::{Widget, SelectableList, Block, Borders},
};

use super::{
    events::Key,
    InputHandler,
};

use crate::{
    graph::{
        CallGraph,
        CallId,
    },
};

pub struct CallGraphWidget<'a>(CallGraphController<'a>);

impl<'a> CallGraphWidget<'a> {
    pub fn new(call_graph: &'a CallGraph) -> Self {
        CallGraphWidget(CallGraphController::new(call_graph))
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

impl<'a> InputHandler for CallGraphWidget<'a> {
    fn handle_input(&mut self, area: Rect, input: &Key) {
        let page_height = area.height as usize;
        match input {
            Key::Down | Key::Char('j') => { self.0.select_next(); }
            Key::Up | Key::Char('k')   => { self.0.select_previous(); }
            Key::Home                  => { self.0.select_first(); }
            Key::End | Key::Char('G')  => { self.0.select_last(); }

            Key::PageDown | Key::Char('f') => { self.0.select_nth_next(page_height); }
            Key::PageUp | Key::Char('b') => { self.0.select_nth_previous(page_height); }

            Key::Left | Key::Char('h')  => {
                if !self.0.are_callers_selected() { self.0.select_callers(); }
                else { self.0.enter_selected(); }
            }
            Key::Right | Key::Char('l') => {
                if !self.0.are_callees_selected() { self.0.select_callees(); }
                else { self.0.enter_selected(); }
            }

            Key::Char('\n') => { self.0.enter_selected(); }
            Key::Backspace  => { self.0.leave_current(); }

            _ => {}
        }
    }
}

// old call graph controller

struct CallGraphController<'a> {
    call_graph: &'a CallGraph,
    callers: CallListController,
    callees: CallListController,
    callees_selected: bool,
    history: Vec<CallId>,
}

impl<'a> CallGraphController<'a> {
    pub fn new(call_graph: &'a CallGraph) -> Self {
        let mut nav = CallGraphController {
            call_graph,
            callees_selected: true,
            callers: CallListController::empty(),
            callees: CallListController::empty(),
            history: vec![],
        };

        nav.navigate_to_root();

        nav
    }

    pub fn select_first(&mut self) {
        self.current_mut().select_first();
    }

    pub fn select_last(&mut self) {
        self.current_mut().select_last();
    }

    pub fn select_next(&mut self) {
        self.current_mut().select_next();
    }

    pub fn select_previous(&mut self) {
        self.current_mut().select_previous();
    }

    pub fn select_nth_next(&mut self, n: usize) {
        self.current_mut().select_nth_next(n);
    }

    pub fn select_nth_previous(&mut self, n: usize) {
        self.current_mut().select_nth_previous(n);
    }

    pub fn callee_list(&self) -> (&CallListController, bool) {
        (&self.callees, self.callees_selected)
    }

    pub fn caller_list(&self) -> (&CallListController, bool) {
        (&self.callers, !self.callees_selected)
    }

    pub fn select_callees(&mut self) {
        self.callees_selected = true;
    }

    pub fn are_callees_selected(&self) -> bool {
        self.callees_selected
    }

    pub fn select_callers(&mut self) {
        self.callees_selected = false;
    }

    pub fn are_callers_selected(&self) -> bool {
        !self.callees_selected
    }

    fn current(&self) -> &CallListController {
        if self.callees_selected {
            &self.callees
        } else {
            &self.callers
        }
    }

    fn current_mut(&mut self) -> &mut CallListController {
        if self.callees_selected {
            &mut self.callees
        } else {
            &mut self.callers
        }
    }

    pub fn enter_selected(&mut self) {
        if let Some(stack) = self.current().selected_item() {
            let call_id = if self.callees_selected { stack.callee_id } else { stack.caller_id };
            self.history.push(call_id);
            self.navigate_to(call_id);
        }
    }

    pub fn leave_current(&mut self) {
        self.history.pop();
        if let Some(&call_id) = self.history.last() {
            self.navigate_to(call_id);
        } else {
            self.navigate_to_root();
        }
    }

    fn navigate_to(&mut self, call_id: CallId) {
        let callers = self.call_graph.neighbors_directed(call_id, Incoming)
            .map(|other_call_id| {
                let (caller_id, callee_id) = (other_call_id, call_id);
                let calls = self.call_graph.edge_weight(caller_id, callee_id).unwrap();
                (caller_id, callee_id, calls.as_slice())
            });
        self.callers = CallListController::new(callers);

        let callees = self.call_graph.neighbors_directed(call_id, Outgoing)
            .map(|other_call_id| {
                let (caller_id, callee_id) = (call_id, other_call_id);
                let calls = self.call_graph.edge_weight(caller_id, callee_id).unwrap();
                (caller_id, callee_id, calls.as_slice())
            });
        self.callees = CallListController::new(callees);
    }

    fn navigate_to_root(&mut self) {
        self.callers = CallListController::empty();

        let callees = self.call_graph.nodes()
            .filter(|&node| self.call_graph.neighbors_directed(node, Incoming).next().is_none())
            .flat_map(|node| self.call_graph.edges(node))
            .map(|(caller_id, callee_id, calls)| (caller_id, callee_id, calls.as_slice()));
        self.callees = CallListController::new(callees);
    }
}

// old call list controller

struct CallListController(SelectionListController<CallStack>);

impl CallListController {
    pub fn new<'a, T>(iter: T) -> Self
        where T: IntoIterator<Item=(CallId, CallId, &'a [usize])>,
    {
        let mut stacks: Vec<_> = iter.into_iter()
            .map(|(caller_id, callee_id, calls)| CallStack::new(caller_id, callee_id, calls.as_ref()))
            .collect();
        stacks.sort_by_key(|stack| stack.allocated_bytes);
        stacks.reverse();
        CallListController(SelectionListController::new(stacks))
    }

    pub fn empty() -> Self {
        CallListController(SelectionListController::new(vec![]))
    }
}

impl Deref for CallListController {
    type Target = SelectionListController<CallStack>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for CallListController {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

pub struct CallStack {
    pub caller_id: CallId,
    pub callee_id: CallId,
    pub description: String,
    pub allocated_bytes: usize,
}

impl CallStack {
    fn new(caller_id: CallId, callee_id: CallId, allocations: &[usize]) -> Self {
        // let location = merge_locations(allocations.iter().map(|alloc| &alloc.location)).unwrap();
        let bytes = allocations.iter().sum();
        CallStack {
            caller_id, callee_id,
            description: format!("{} bytes", bytes),
            allocated_bytes: bytes,
        }
    }
}

impl AsRef<str> for CallStack {
    fn as_ref(&self) -> &str {
        &self.description
    }
}

// old selection list controller

struct SelectionListController<T: AsRef<str>> {
    items: Vec<T>,
    selected: Option<usize>,
}

impl<T: AsRef<str>> SelectionListController<T> {
    pub fn new(items: Vec<T>) -> Self {
        let selected = if !items.is_empty() { Some(0) } else { None };
        SelectionListController { items, selected }
    }

    pub fn select_first(&mut self) {
        if let Some(selected) = self.selected.as_mut() {
            *selected = 0;
        }
    }

    pub fn select_last(&mut self) {
        if let Some(selected) = self.selected.as_mut() {
            *selected = self.items.len() - 1;
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
            *i += cmp::min(self.items.len()-1-*i, n);
        }
    }

    pub fn select_nth_previous(&mut self, n: usize) {
        if let Some(i) = self.selected.as_mut() {
            *i -= cmp::min(*i, n);
        }
    }

    pub fn items(&self) -> &[T] {
        &self.items[..]
    }

    pub fn selected_index(&self) -> Option<usize> {
        self.selected
    }

    pub fn selected_item(&self) -> Option<&T> {
        self.selected.as_ref()
            .map(|&i| &self.items[i])
    }
}
