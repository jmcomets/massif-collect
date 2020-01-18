use std::fs::File;
use std::io::{self, Write};
use std::panic;
use std::path::Path;

#[allow(unused_imports)]
use tui::{
    Terminal,
    backend::{TermionBackend},
    widgets::Widget,
    style::{Style, Color},
    layout::{Layout, Constraint, Direction, Rect},
    buffer::Buffer,
};

use termion::{
    event::Key,
    input::MouseTerminal,
    raw::IntoRawMode,
    screen::{AlternateScreen, ToMainScreen},
};

use crate::{
    Snapshot,
    graph::CallGraph,
};

mod events;

mod allocation_graph;
// mod caller_tree;
mod call_graph;

#[allow(unused_imports)]
use self::{
    events::{Events, Event},
    call_graph::CallGraphWidget,
    allocation_graph::AllocationGraphWidget,
};

pub trait InputHandler {
    fn handle_input(&mut self, area: Rect, input: &Key);
}

// impl<'a> InputHandler for CallerTreeController<'a> {
//     fn handle_input(&mut self, area: Rect, input: &Key) {
//         let page_height = area.height as usize;
//         match input {
//             Key::Down | Key::Char('j') => { self.select_next(page_height); }
//             Key::Up | Key::Char('k')   => { self.select_previous(); }
//             Key::Home                  => { self.reset(); }
//             Key::Char('\n')            => { self.toggle_selected(); }

//             Key::PageDown | Key::Char('f') => { self.select_nth_next(page_height, page_height); }
//             Key::PageUp | Key::Char('b') => { self.select_nth_previous(page_height); }

//             _ => {}
//         }
//     }
// }

pub fn run<P: AsRef<Path>>(output: Option<P>, snapshots: &[Snapshot]) -> io::Result<()> {
    let output: Box<dyn Write> = if let Some(path) = output {
        let file = File::create(path.as_ref())?;
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

    let ref caller_tree = snapshots[0].tree;
    let call_graph = CallGraph::from_tree(caller_tree);

    // let mut allocation_graph = AllocationGraphWidget::new(&snapshots);
    let mut call_graph = CallGraphWidget::new(&call_graph);
    // let mut caller_tree = CallerTreeWidget::new(&caller_tree);

    loop {
        terminal.draw(|mut f| {
            let size = f.size();
            // allocation_graph.render(&mut f, size);
            call_graph.render(&mut f, size);
            // caller_tree.render(&mut f, size);
        })?;

        // since output is buffered, flush to see the effect immediately when hitting backspace
        io::stdout().flush().ok();

        macro_rules! io_error {
            ($tag:expr, $e:expr) => {{
                let message = format!("{}: {:?}", $tag, $e);
                ::std::io::Error::new(::std::io::ErrorKind::Other, message)
            }}
        }

        match events.next().map_err(|e| io_error!("handling events", e))? {
                Event::Input(input) => {
                    let size = terminal.size().unwrap();
                    match input {
                        Key::Char('q') => { break; }
                        input @ _      => {
                            // caller_tree.handle_input(size, &input);
                            call_graph.handle_input(size, &input);
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
