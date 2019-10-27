use std::io::{self, Write};
use std::panic;

use tui::{
    Terminal,
    backend::TermionBackend,
};

use termion::{
    event::Key,
    input::MouseTerminal,
    raw::IntoRawMode,
    screen::{AlternateScreen, ToMainScreen},
};

use crate::{
    CallGraph,
    CallerTree,
};

mod controllers;
mod events;
mod views;

use self::{
    controllers::{
        CallGraphController,
        CallerTreeController,
    },
    events::{Events, Event},
    views::{render_caller_tree, render_call_graph},
};

macro_rules! io_error {
    ($tag:expr) => {{
        |e| {
            let message = format!("{}: {:?}", $tag, e);
            io::Error::new(io::ErrorKind::Other, message)
        }
    }}
}

enum Tab {
    CallerTree,
    CallGraph,
}

impl Tab {
    fn next(self) -> Self {
        match self {
            Tab::CallerTree => Tab::CallGraph,
            Tab::CallGraph  => Tab::CallerTree,
        }
    }
}

pub fn run(caller_tree: &CallerTree, call_graph: &CallGraph) -> io::Result<()> {
    let stdout = io::stdout().into_raw_mode()?;
    let stdout = MouseTerminal::from(stdout);
    let stdout = AlternateScreen::from(stdout);

    let backend = TermionBackend::new(stdout);
    set_termion_panic_hook();

    let mut terminal = Terminal::new(backend)?;

    let events = Events::new();

    let caller_tree = CallerTreeController::new(&caller_tree);
    let mut call_graph = CallGraphController::new(&call_graph);

    let mut tab = Tab::CallerTree;

    loop {
        terminal.draw(|mut f| {
            match tab {
                Tab::CallerTree => render_caller_tree(&caller_tree, &mut f),
                Tab::CallGraph  => render_call_graph(&call_graph, &mut f),
            }
        })?;

        // stdout is buffered, flush it to see the effect immediately when hitting backspace
        io::stdout().flush().ok();

        let size = terminal.size().unwrap();

        match events.next().map_err(io_error!("handling events"))? {
            Event::Input(input) => match input {
                Key::Char('q') => { break; }

                Key::Down | Key::Char('j') => { call_graph.select_next(); }
                Key::Up | Key::Char('k')   => { call_graph.select_previous(); }
                Key::Home                  => { call_graph.select_first(); }
                Key::End | Key::Char('G')  => { call_graph.select_last(); }

                Key::PageDown | Key::Char('f') => { call_graph.select_nth_next(size.height as usize); }
                Key::PageUp | Key::Char('b')   => { call_graph.select_nth_previous(size.height as usize); }

                Key::Left | Key::Char('h')  => {
                    if !call_graph.are_callers_selected()
                    {
                        call_graph.select_callers();
                    }
                    else
                    {
                        call_graph.enter_selected();
                    }
                }
                Key::Right | Key::Char('l') => {
                    if !call_graph.are_callees_selected()
                    {
                        call_graph.select_callees();
                    }
                    else
                    {
                        call_graph.enter_selected();
                    }
                }

                Key::Char('\n') => { call_graph.enter_selected(); }
                Key::Backspace  => { call_graph.leave_current(); }

                Key::Char('\t') => { tab = tab.next(); }

                _ => {}
            },
            _ => {}
        }
    }

    Ok(())
}

fn set_termion_panic_hook() {
    panic::set_hook(Box::new(|info| {
        let location = info.location().unwrap(); // The current implementation always returns Some

        let msg = match info.payload().downcast_ref::<&'static str>() {
            Some(s) => *s,
            None => match info.payload().downcast_ref::<String>() {
                Some(s) => &s[..],
                None => "Box<Any>",
            }
        };

        eprintln!("{}thread '<unnamed>' panicked at '{}', {}\r", ToMainScreen, msg, location);
    }));
}
