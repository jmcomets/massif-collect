use std::fs::File;
use std::io::{self, Write};
use std::panic;
use std::path::Path;

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
    graph::build_call_graph,
};

mod events;

// mod input;
// mod controllers;
// mod views;

use self::{
    // controllers::{
    //     CallGraphController,
    //     CallerTreeController,
    // },
    events::{Events, Event},
    // input::InputHandler,
    // views::{CallerTreeWidget, CallGraphWidget},
};

struct AllocationGraphWidget<'a> {
    snapshots: &'a [Snapshot],
}

impl<'a> AllocationGraphWidget<'a> {
    fn new(snapshots: &'a [Snapshot]) -> Self {
        AllocationGraphWidget { snapshots }
    }
}

fn pad_area(mut area: Rect, percentage: u16) -> Rect {
    debug_assert!(percentage <= 100);

    let constraints = [
        Constraint::Percentage(percentage),
        Constraint::Percentage(100 - 2 * percentage),
        Constraint::Percentage(percentage)
    ];

    for direction in [Direction::Horizontal, Direction::Vertical].into_iter().cloned() {
        area = Layout::default()
            .direction(direction)
            .constraints(constraints.as_ref())
            .split(area)[1];
    }

    area
}

impl<'a> Widget for AllocationGraphWidget<'a> {
    fn draw(&mut self, area: Rect, buf: &mut Buffer) {
        let area = pad_area(area, 10);

        let max_bytes = self.snapshots.iter()
            .map(|snapshot| snapshot.tree.sample.bytes)
            .max().unwrap_or(0);

        for (i, snapshot) in self.snapshots.iter().enumerate() {
            let bar_width = (i * area.width as usize / self.snapshots.len()) as u16;
            let bar_height = (snapshot.tree.sample.bytes * area.height as usize / max_bytes) as u16;

            // TODO use a rect
            for x in 0..bar_width {
                for y in 0..bar_height {
                    buf.set_string(x + area.left(), y + area.top(), " ", Style::default().bg(Color::Red));
                }
            }
        }
    }
}

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

    // let ref caller_tree = snapshots[0].tree;
    // let call_graph = build_call_graph(caller_tree);

    // let mut caller_tree = CallerTreeController::new(&caller_tree);
    // let mut call_graph = CallGraphController::new(&call_graph);

    loop {
        terminal.draw(|mut f| {
            let size = f.size();
            AllocationGraphWidget::new(&snapshots).render(&mut f, size);
            // CallerTreeWidget::new(&caller_tree).render(&mut f, size);
            // CallGraphWidget::new(&call_graph).render(&mut f, size);
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
                        input @ _       => {
                            // caller_tree.handle_input(size, &input);
                            // call_graph.handle_input(size, &input);
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
