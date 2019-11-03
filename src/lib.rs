use std::collections::{HashMap, HashSet};
use std::io::{self, BufRead};

#[macro_use]
extern crate nom;

use petgraph::graphmap::DiGraphMap;

pub mod ui;

mod parsing;
mod indexing;
mod iters;

pub type CallId = usize;

#[derive(Debug, Default)]
pub struct CallerTree(HashMap<CallId, CallerTree>);

impl CallerTree {
    fn add_caller_tree(&mut self, caller_id: CallId, caller_tree: CallerTree) {
        self.0.insert(caller_id, caller_tree);
    }

    pub fn iter(&self) -> impl Iterator<Item=(CallId, &CallerTree)> {
        self.0.iter().map(|(&k, v)| (k, v))
    }

    #[inline]
    pub fn walk_callers(&self) -> impl Iterator<Item=(CallId, &CallerTree, usize)> {
        self.walk_callers_box(None, 0)
    }

    fn walk_callers_box(&self, caller_id: Option<CallId>, depth: usize) -> Box<dyn Iterator<Item=(CallId, &CallerTree, usize)> + '_> {
        let node = caller_id.map(|caller_id| (caller_id, self, depth));
        let subtree = self.0.iter().flat_map(move |(&caller_id, caller)| caller.walk_callers_box(Some(caller_id), depth+1));
        Box::new(iters::PrefixedIter::new(node, subtree))
    }

    // fn walk_callers_iter(&self) -> impl Iterator<Item=(CallId, &CallerTree)> {
    //     unimplemented!()
    // }
}

#[derive(Debug, Default)]
struct CallerTreeBuilder{
    root: CallerTree,
    caller_stack: Vec<(CallId, CallerTree)>,
    partial_trees: HashSet<CallId>,
}

impl CallerTreeBuilder {
    fn add_call(&mut self, caller_id: CallId, callee_id: CallId) -> Result<(), CycleDetected> {
        if self.partial_trees.contains(&caller_id) {
            return Err(CycleDetected(caller_id, callee_id));
        }

        // the callee should be on top of the caller_stack
        self.unwind_until(Some(callee_id));
        if self.caller_stack.is_empty() {
            self.caller_stack.push((callee_id, CallerTree::default()));
            self.partial_trees.insert(callee_id);
        }

        self.caller_stack.push((caller_id, CallerTree::default()));
        // note that we don't mark the caller as "partial" to allow sibling calls

        Ok(())
    }

    fn unwind_until(&mut self, callee_id: Option<CallId>) {
        while let Some(call_id) = self.caller_stack.last().map(|(call_id, _)| *call_id) {
            if Some(call_id) == callee_id {
                break;
            }

            self.partial_trees.remove(&call_id);

            let (_, caller_tree) = self.caller_stack.pop().unwrap();
            let callee_tree = self.caller_stack.last_mut()
                .map(|(_, callee_tree)| callee_tree)
                .unwrap_or(&mut self.root);

            callee_tree.add_caller_tree(call_id, caller_tree);
        }
    }

    fn build(mut self) -> CallerTree {
        self.unwind_until(None);
        self.root
    }
}

#[derive(Debug)]
struct CycleDetected(CallId, CallId);

pub type CallGraph = DiGraphMap<CallId, Vec<Allocation>>;

pub fn read_massif<R: BufRead>(reader: R) -> io::Result<(CallerTree, CallGraph)> {
    let mut call_index = indexing::CallIndex::new();

    let mut caller_tree_builder = CallerTreeBuilder::default();
    let mut call_graph = CallGraph::new();

    for entry in parsing::massif_tree(reader) {
        let (caller, callee, allocation) = entry?;

        let callee_id = {
            if let Some(callee) = callee {
                call_index.index(callee)
            } else {
                call_index.index_leaf_callee()
            }
        };

        let caller_id = call_index.index(caller);

        caller_tree_builder.add_call(caller_id, callee_id).unwrap();

        call_graph.edge_entry(caller_id, callee_id)
            .or_insert(vec![])
            .push(allocation);
    }

    let caller_tree = caller_tree_builder.build();

    Ok((caller_tree, call_graph))
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
