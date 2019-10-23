#![allow(unused)]

use std::mem;
use std::env;
use std::fs::File;
use std::io::{self, BufReader, Write};

use petgraph::prelude::*;
use petgraph::visit::{depth_first_search, DfsEvent, Control};

use tui::{
    Terminal,
    backend::{Backend, TermionBackend},
    widgets::{Widget, Block, Borders},
    layout::{Layout, Constraint, Direction, Rect},
    terminal::Frame,
};
use termion::{
    cursor::Goto,
    event::Key,
    input::MouseTerminal,
    raw::IntoRawMode,
    screen::AlternateScreen,
};

use tui::style::{Color, Modifier, Style};
use tui::widgets::{List, Text, SelectableList};

mod events;

macro_rules! io_error {
    ($tag:expr) => {{
        |e| {
            let message = format!("{}: {:?}", $tag, e);
            io::Error::new(io::ErrorKind::Other, message)
        }
    }}
}

use crate::events::{Event, Events};

use massif_collect::{read_massif, CallGraph, Allocation};

struct CallList {
    stacks: Vec<CallStack>,
    selected: Option<usize>,
}

impl CallList {
    fn new(mut stacks: Vec<CallStack>) -> Self {
        stacks.sort_by_key(|stack| stack.allocated_bytes);
        stacks.reverse();

        CallList {
            stacks,
            selected: Some(0),
        }
    }

    fn first(&mut self) {
        if let Some(selected) = self.selected.as_mut() {
            *selected = 0;
        }
    }

    fn last(&mut self) {
        if let Some(selected) = self.selected.as_mut() {
            *selected = self.stacks.len() - 1;
        }
    }

    fn next(&mut self) {
        if let Some(selected) = self.selected.as_mut() {
            if *selected+1 < self.stacks.len() {
                *selected += 1;
            }
        }
    }

    fn prev(&mut self) {
        if let Some(selected) = self.selected.as_mut() {
            if *selected > 0 {
                *selected -= 1;
            }
        }
    }

    fn selection(&self) -> Option<&CallStack> {
        self.selected.as_ref()
            .map(|&i| &self.stacks[i])
    }
}

struct CallStack {
    caller_id: usize,
    callee_id: usize,
    description: String,
    allocated_bytes: usize,
}

impl CallStack {
    fn new(caller_id: usize, callee_id: usize, allocation: &Allocation) -> Self {
        CallStack {
            caller_id, callee_id,
            description: format!("{}: {}", allocation.bytes, allocation.location.to_string()),
            allocated_bytes: allocation.bytes,
        }
    }
}

impl AsRef<str> for CallStack {
    fn as_ref(&self) -> &str {
        &self.description
    }
}

struct App<'a> {
    call_graph: &'a CallGraph,
    callees_selected: bool,
    call_lists: Vec<CallList>,
}

impl<'a> App<'a> {
    fn new(call_graph: &'a CallGraph) -> Self {
        let root_stacks: Vec<_> = call_graph.nodes()
            .filter(|&node| call_graph.neighbors_directed(node, Incoming).next().is_none())
            .flat_map(|node| call_graph.edges(node))
            .map(|(caller_id, callee_id, allocation)| CallStack::new(caller_id, callee_id, allocation))
            .collect();

        App {
            call_graph,
            callees_selected: true,
            call_lists: vec![CallList::new(vec![]), CallList::new(root_stacks)],
        }
    }

    fn selection(&self) -> &CallList {
        let mut i = self.call_lists.len()-2;
        if self.callees_selected { i += 1; }
        &self.call_lists[i]
    }

    fn selection_mut(&mut self) -> &mut CallList {
        let mut i = self.call_lists.len()-2;
        if self.callees_selected { i += 1; }
        &mut self.call_lists[i]
    }

    fn first(&mut self) {
        self.selection_mut().first();
    }

    fn last(&mut self) {
        self.selection_mut().last();
    }

    fn next(&mut self) {
        self.selection_mut().next();
    }

    fn prev(&mut self) {
        self.selection_mut().prev();
    }

    fn left(&mut self) {
        self.callees_selected = false;
    }

    fn right(&mut self) {
        self.callees_selected = true;
    }

    fn enter(&mut self) {
        if let Some(stack) = self.selection().selection() {
            let (call_id, direction) = if self.callees_selected {
                (stack.callee_id, Outgoing)
            } else {
                (stack.caller_id, Incoming)
            };

            let stacks = self.call_graph.neighbors_directed(call_id, direction)
                .map(|other_call_id| {
                    let (caller_id, callee_id) = if direction == Outgoing {
                        (call_id, other_call_id)
                    } else {
                        (other_call_id, call_id)
                    };

                    let allocation = self.call_graph.edge_weight(caller_id, callee_id).unwrap();
                    CallStack::new(caller_id, callee_id, allocation)
                })
                .collect();

            self.call_lists.push(CallList::new(stacks));
        }
    }

    fn leave(&mut self) {
        if self.call_lists.len() > 2 {
            self.call_lists.pop();
        }
    }

    fn callers(&self) -> (&CallList, bool) {
        let i = self.call_lists.len()-2;
        (&self.call_lists[i], !self.callees_selected)
    }

    fn callees(&self) -> (&CallList, bool) {
        let i = self.call_lists.len()-1;
        (&self.call_lists[i], self.callees_selected)
    }
}

fn call_list_widget<'a>(title: &'a str, (call_list, active): (&'a CallList, bool)) -> SelectableList<'a> {
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
        .items(&call_list.stacks)
        .select(call_list.selected)
        .style(default_style)
        .highlight_style(highlight_style)
        .highlight_symbol(">")
}

fn main() -> io::Result<()> {
    let filename = env::args().nth(1).unwrap_or("data/example.out".to_string());
    let file = File::open(filename)?;
    let reader = BufReader::new(file);
    let call_graph = read_massif(reader)?;

    let stdout = io::stdout().into_raw_mode()?;
    let stdout = MouseTerminal::from(stdout);
    let stdout = AlternateScreen::from(stdout);
    let backend = TermionBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    set_termion_panic_hook();

    let events = Events::new();

    let mut app = App::new(&call_graph);

    let default_style = Style::default().fg(Color::White).bg(Color::Black);

    loop {
        terminal.draw(|mut f| {
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

            call_list_widget("Callers", app.callers()).render(&mut f, chunks[0]);
            call_list_widget("Callees", app.callees()).render(&mut f, chunks[1]);
        })?;

        // stdout is buffered, flush it to see the effect immediately when hitting backspace
        io::stdout().flush().ok();

        match events.next().map_err(io_error!("handling events"))? {
            Event::Input(input) => match input {
                Key::Char('q') => { break; }

                Key::Down | Key::Char('j') => { app.next(); }
                Key::Up | Key::Char('k')   => { app.prev(); }
                Key::Home                  => { app.first(); }
                Key::End | Key::Char('G')  => { app.last(); }

                Key::Left | Key::Char('h')  => { app.left(); }
                Key::Right | Key::Char('l') => { app.right(); }


                Key::Char('\n') => { app.enter(); }
                Key::Backspace  => { app.leave(); }

                _ => {}
            },
            _ => {}
        }
    }

    Ok(())
}

fn set_termion_panic_hook() {
    std::panic::set_hook(Box::new(|info| {
        let location = info.location().unwrap(); // The current implementation always returns Some

        let msg = match info.payload().downcast_ref::<&'static str>() {
            Some(s) => *s,
            None => match info.payload().downcast_ref::<String>() {
                Some(s) => &s[..],
                None => "Box<Any>",
            }
        };

        println!("{}thread '<unnamed>' panicked at '{}', {}\r", termion::screen::ToMainScreen, msg, location);
    }));
}
