use std::io::{self, Write};
use std::panic;

use tui::{
    Terminal,
    backend::TermionBackend,
    layout::{Layout, Constraint, Direction},
    style::{Color, Style},
    widgets::{Widget, SelectableList, Block, Borders},
};

use termion::{
    event::Key,
    input::MouseTerminal,
    raw::IntoRawMode,
    screen::{AlternateScreen, ToMainScreen},
};

use crate::{
    CallGraph,
    components::{CallGraphNavigator, CallList},
    events::{Events, Event},
};

macro_rules! io_error {
    ($tag:expr) => {{
        |e| {
            let message = format!("{}: {:?}", $tag, e);
            io::Error::new(io::ErrorKind::Other, message)
        }
    }}
}

pub fn navigate_call_graph(call_graph: &CallGraph) -> io::Result<()> {
    let stdout = io::stdout().into_raw_mode()?;
    let stdout = MouseTerminal::from(stdout);
    let stdout = AlternateScreen::from(stdout);

    let backend = TermionBackend::new(stdout);
    set_termion_panic_hook();

    let mut terminal = Terminal::new(backend)?;

    let events = Events::new();

    let mut app = CallGraphNavigator::new(&call_graph);

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

            let (callers, callers_active) = app.caller_list();
            call_list_widget("Callers", callers, callers_active).render(&mut f, chunks[0]);
            let (callees, callees_active) = app.callee_list();
            call_list_widget("Callees", callees, callees_active).render(&mut f, chunks[1]);
        })?;

        // stdout is buffered, flush it to see the effect immediately when hitting backspace
        io::stdout().flush().ok();

        match events.next().map_err(io_error!("handling events"))? {
            Event::Input(input) => match input {
                Key::Char('q') => { break; }

                Key::Down | Key::Char('j') => { app.select_next(); }
                Key::Up | Key::Char('k')   => { app.select_previous(); }
                Key::Home                  => { app.select_first(); }
                Key::End | Key::Char('G')  => { app.select_last(); }

                Key::Left | Key::Char('h')  => { app.select_callers(); }
                Key::Right | Key::Char('l') => { app.select_callees(); }

                Key::Char('\n') => { app.enter_selected(); }
                Key::Backspace  => { app.leave_current(); }

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

fn call_list_widget<'a>(title: &'a str, call_list: &'a CallList, active: bool) -> SelectableList<'a> {
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
