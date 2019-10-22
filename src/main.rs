#![allow(unused)]
use std::env;
use std::fs::File;
use std::io::{self, BufReader, Write};

use petgraph::prelude::*;
use petgraph::visit::{depth_first_search, DfsEvent, Control};

use tui::{
    Terminal,
    backend::TermionBackend,
    widgets::{Widget, Block, Borders},
    layout::{Layout, Constraint, Direction},
};
use termion::raw::IntoRawMode;


use termion::cursor::Goto;
use termion::event::Key;
use termion::input::MouseTerminal;
use termion::screen::AlternateScreen;
use tui::style::{Color, Modifier, Style};
use tui::widgets::{List, Text, SelectableList};

mod events;

#[macro_use]
mod macros;

use crate::events::{Event, Events};

use massif_collect::{read_massif, CallGraph};

fn main() -> io::Result<()> {
    // Terminal initialization
    let stdout = io::stdout().into_raw_mode()?;
    let stdout = MouseTerminal::from(stdout);
    let stdout = AlternateScreen::from(stdout);
    let backend = TermionBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Setup event handlers
    let events = Events::new();

    let mut input = String::new();

    let callers = vec!["Apples", "Oranges", "Plums", "Raisins"];
    let mut selected_caller = Some(0);

    let callees = vec!["Apples", "Oranges", "Plums", "Raisins"];
    let mut selected_callee = Some(0);

    let mut on_callers = true;

    let default_style = Style::default().fg(Color::White).bg(Color::Black);
    let inactive_highlight_style = default_style.fg(Color::Gray).bg(Color::Black);
    let active_highlight_style = default_style.fg(Color::Black).bg(Color::White);

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

            let caller_style = if on_callers { active_highlight_style } else { inactive_highlight_style };
            SelectableList::default()
                .block(Block::default().borders(Borders::ALL).title("Callers"))
                .items(&callers)
                .select(selected_caller)
                .style(default_style)
                .highlight_style(caller_style)
                .highlight_symbol(">")
                .render(&mut f, chunks[0]);

            let callee_style = if on_callers { inactive_highlight_style } else { active_highlight_style };
            SelectableList::default()
                .block(Block::default().borders(Borders::ALL).title("Callees"))
                .items(&callees)
                .select(selected_callee)
                .style(default_style)
                .highlight_style(callee_style)
                .highlight_symbol(">")
                .render(&mut f, chunks[1]);
            })?;

        // stdout is buffered, flush it to see the effect immediately when hitting backspace
        io::stdout().flush().ok();

        match events.next().map_err(io_error!("handling events"))? {
            Event::Input(input) => match input {
                Key::Char('q') => {
                    break;
                }

                Key::Char('j') => {
                    if on_callers {
                        let selected_caller = selected_caller.as_mut().unwrap();
                        if *selected_caller+1 < callers.len() {
                            *selected_caller += 1;
                        }
                    } else {
                        let selected_callee = selected_callee.as_mut().unwrap();
                        if *selected_callee+1 < callees.len() {
                            *selected_callee += 1;
                        }
                    }
                }

                Key::Char('k') => {
                    if on_callers {
                        let selected_caller = selected_caller.as_mut().unwrap();
                        if *selected_caller > 0 {
                            *selected_caller -= 1;
                        }
                    } else {
                        let selected_callee = selected_callee.as_mut().unwrap();
                        if *selected_callee > 0 {
                            *selected_callee -= 1;
                        }
                    }
                }

                Key::Char('h') => {
                    on_callers = true;
                }

                Key::Char('l') => {
                    on_callers = false;
                }

                Key::Char('\n') => {
                    // TODO enter selected
                }

                Key::Backspace => {
                    // TODO move back in history
                }
                _ => {}
            },
            _ => {}
        }
    }

    // let filename = env::args().nth(1).unwrap_or("data/example.out".to_string());
    // let file = File::open(filename)?;
    // let reader = BufReader::new(file);
    // let call_graph = read_massif(reader)?;

    // debug_call_graph(&call_graph);

    Ok(())
}

fn debug_call_graph(call_graph: &CallGraph) {
    // FIXME the allocations of the callers should sum up to the allocation of the callee
    #[cfg(debug_assertions)] {
        for callee1_id in call_graph.nodes() {
            for caller1_id in call_graph.neighbors_directed(callee1_id, Incoming) {
                let allocation1 = call_graph.edge_weight(caller1_id, callee1_id).unwrap();

                let callee2_id = caller1_id;
                if call_graph.neighbors_directed(callee2_id, Incoming).next().is_none() {
                    continue;
                }

                let alloc1 = allocation1.bytes;

                let mut alloc2 = 0;
                for caller2_id in call_graph.neighbors_directed(callee2_id, Incoming) {
                    let allocation2 = call_graph.edge_weight(caller2_id, callee2_id).unwrap();
                    alloc2 += allocation2.bytes;
                }

                assert!(alloc2 <= alloc1, "* -> {} ; {} -> {} ({} vs {})", caller1_id, caller1_id, callee1_id, alloc2, alloc1);

                let untracked = alloc1 - alloc2;
                if untracked > 0 {
                    println!("untracked alloc of {} bytes in {} -> {}", untracked, caller1_id, callee1_id);
                }
            }
        }
    }

    for root_call_id in call_graph.nodes() {
        // only consider roots
        if call_graph.neighbors_directed(root_call_id, Incoming).next().is_some() {
            continue;
        }

        let mut depth = 0;
        depth_first_search(&call_graph, Some(root_call_id), |event| {
            use DfsEvent::*;
            match event {
                Discover(_, _) => { depth += 1; }
                Finish(_, _)   => { depth -= 1; }
                TreeEdge(caller_id, callee_id) | BackEdge(caller_id, callee_id) | CrossForwardEdge(caller_id, callee_id) => {
                    let allocation = call_graph.edge_weight(caller_id, callee_id).unwrap();
                    let bytes = allocation.bytes;

                    // since the edges already aggregate the bytes allocated through call, this needn't be a DFS
                    // TODO cache this sum?
                    let mut total_call_bytes = 0;
                    for (_, _, allocation) in call_graph.edges(caller_id) {
                        total_call_bytes += allocation.bytes;
                    }

                    let call_ratio = 100. * (bytes as f64) / (total_call_bytes as f64);
                    // let root_ratio = 100. * (bytes as f64) / (total_root_bytes as f64);

                    for _ in 0..depth { print!(" "); }
                    print!("{}", bytes);
                    // print!(" (");
                    if bytes < total_call_bytes {
                        print!(" (");
                        print!("{:.2}% of {} [call]", call_ratio, total_call_bytes);
                        // print!(", ");
                        print!(")");
                    }
                    // print!("{:.2}% of {} [total]", root_ratio, total_root_bytes);
                    // print!(")");
                    print!(": {}", allocation.location.to_string());
                    println!();
                }
            }

            Control::<()>::Continue
        });
    }
}
