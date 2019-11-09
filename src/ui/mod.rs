use std::fs::File;
use std::io::{self, Write};
use std::panic;

use tui::{
    Terminal,
    backend::{TermionBackend},
    widgets::Widget,
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
mod input;
mod views;

use self::{
    controllers::{
        CallGraphController,
        CallerTreeController,
    },
    events::{Events, Event},
    input::InputHandler,
    views::{CallerTreeWidget, CallGraphWidget},
};

macro_rules! io_error {
    ($tag:expr) => {{
        |e| {
            let message = format!("{}: {:?}", $tag, e);
            io::Error::new(io::ErrorKind::Other, message)
        }
    }}
}

pub fn run(output: Option<&str>, caller_tree: &CallerTree, call_graph: &CallGraph) -> io::Result<()> {
    let output: Box<dyn Write> = if let Some(filename) = output {
        let file = File::create(filename)?;
        Box::new(file)
    } else {
        let stdout = io::stdout();
        Box::new(stdout)
    };

    let output = output.into_raw_mode()?;
    let output = MouseTerminal::from(output);
    let output = AlternateScreen::from(output);

    let backend = TermionBackend::new(output);
    set_termion_panic_hook();

    let mut terminal = Terminal::new(backend)?;

    let events = Events::new();

    let mut caller_tree = CallerTreeController::new(&caller_tree);
    let mut call_graph = CallGraphController::new(&call_graph);

    let mut tab_index = 0;

    loop {
        terminal.draw(|mut f| {
            let size = f.size();
            if tab_index == 0 {
                CallerTreeWidget::new(&caller_tree).render(&mut f, size);
            } else {
                CallGraphWidget::new(&call_graph).render(&mut f, size);
            }
        })?;

        // since output is buffered, flush to see the effect immediately when hitting backspace
        io::stdout().flush().ok();

        match events.next().map_err(io_error!("handling events"))? {
                Event::Input(input) => {
                    let size = terminal.size().unwrap();
                    match input {
                        Key::Char('q') => { break; }
                        Key::Char('\t') => { tab_index = (tab_index + 1) % 2 }
                        input @ _       => {
                            if tab_index == 0 {
                                caller_tree.handle_input(size, &input);
                            } else {
                                call_graph.handle_input(size, &input);
                            }
                        }
                    }
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
