use std::io::{self, BufRead};

#[macro_use]
extern crate nom;

use petgraph::graphmap::DiGraphMap;

mod app;
mod components;
mod events;

mod parsing;
mod indexing;

pub type CallId = usize;

pub type CallGraph = DiGraphMap<CallId, Allocation>;

pub use app::navigate_call_graph;

pub fn read_massif<R: BufRead>(reader: R) -> io::Result<CallGraph> {
    let mut call_index = indexing::CallIndex::new();
    let mut call_graph = CallGraph::new();

    for entry in parsing::massif_tree(reader) {
        let (caller, callee, allocation) = entry?;

        let callee_id = {
            if let Some(callee) = callee {
                call_index.index(callee)
            } else {
                call_index.index_leaf_caller()
            }
        };

        let caller_id = call_index.index(caller);

        call_graph.add_edge(caller_id, callee_id, allocation);
    }

    Ok(call_graph)
}

#[derive(Debug, Clone, PartialEq)]
pub enum Call {
    Inner(String),
    Leaf,
    Root,
}

#[derive(Debug, PartialEq)]
pub struct Allocation {
    pub bytes: usize,
    pub location: Location,
}

impl Allocation {
    fn new(bytes: usize, location: Location) -> Self {
        Allocation { bytes, location }
    }
}

#[derive(Debug, PartialEq)]
pub enum Location {
    Described(String),
    Omitted((usize, f32)),
}

impl ToString for Location {
    fn to_string(&self) -> String {
        use Location::*;
        match self {
            Described(description)      => description.clone(),
            Omitted((count, threshold)) => {
                let plural = if count > &1 { "s" } else { "" };
                format!("in {} place{}, below massif's threshold ({:.2}%)", count, plural, threshold)
            }
        }
    }
}
