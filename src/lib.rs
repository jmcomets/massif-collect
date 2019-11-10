use std::collections::HashSet;
use std::io::{self, BufRead};

#[macro_use]
extern crate nom;

use petgraph::graphmap::DiGraphMap;

pub mod ui;

mod parsing;
mod indexing;

pub type CallId = usize;

#[derive(Debug, Default)]
pub struct CallerTree(Vec<(CallId, CallerTreeNode)>);

impl CallerTree {
    pub fn iter(&self) -> impl DoubleEndedIterator<Item=&(CallId, CallerTreeNode)> {
        self.0.iter()
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }
}

#[derive(Debug, Default)]
pub struct CallerTreeNode(Vec<(CallId, CallerTreeNode, Allocation)>);

impl CallerTreeNode {
    pub fn iter(&self) -> impl DoubleEndedIterator<Item=(CallId, &CallerTreeNode, &Allocation)> {
        self.0.iter().map(|(id, node, allocation)| (*id, node, allocation))
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }
}

#[derive(Debug, Default)]
struct CallerTreeBuilder{
    tree: CallerTree,
    caller_stack: Vec<(CallId, CallerTreeNode)>,
    allocations: Vec<Allocation>,
    partial_trees: HashSet<CallId>,
}

impl CallerTreeBuilder {
    fn add_call(&mut self, caller_id: CallId, callee_id: CallId, allocation: Allocation) -> Result<(), CycleDetected> {
        if self.partial_trees.contains(&caller_id) {
            return Err(CycleDetected(caller_id, callee_id));
        }

        // the callee should be on top of the caller_stack
        self.unwind_until(Some(callee_id));
        if self.caller_stack.is_empty() {
            self.caller_stack.push((callee_id, CallerTreeNode::default()));
            self.partial_trees.insert(callee_id);
        }

        self.caller_stack.push((caller_id, CallerTreeNode::default()));
        self.allocations.push(allocation);
        debug_assert_eq!(self.allocations.len() + 1, self.caller_stack.len());

        // note that we don't mark the caller as "partial" to allow sibling calls

        Ok(())
    }

    fn unwind_until(&mut self, callee_id: Option<CallId>) {
        while let Some(call_id) = self.caller_stack.last().map(|(call_id, _)| *call_id) {
            if Some(call_id) == callee_id {
                break;
            }

            self.partial_trees.remove(&call_id);

            let (_, mut caller_tree_node) = self.caller_stack.pop().unwrap();
            caller_tree_node.0.sort_by_key(|(id, _, _)| *id);

            if let Some((_, callee_tree_node)) = self.caller_stack.last_mut() {
                let allocation = self.allocations.pop().unwrap();
                callee_tree_node.0.push((call_id, caller_tree_node, allocation));
            } else {
                self.tree.0.push((call_id, caller_tree_node));
            }
        }
    }

    fn build(mut self) -> CallerTree {
        self.unwind_until(None);
        self.tree
    }
}

#[derive(Debug)]
struct CycleDetected(CallId, CallId);

pub type CallGraph = DiGraphMap<CallId, Vec<Allocation>>;

pub fn read_massif<R: BufRead>(reader: R) -> io::Result<(CallerTree, CallGraph)> {
    let mut call_index = indexing::CallIndex::new();

    let mut caller_tree_node_builder = CallerTreeBuilder::default();
    let mut call_graph = CallGraph::new();

    for entry in parsing::read_massif_tree(reader) {
        let (caller, callee, allocation) = entry?;

        let callee_id = {
            if let Some(callee) = callee {
                call_index.index(callee)
            } else {
                call_index.index_leaf_callee()
            }
        };

        let caller_id = call_index.index(caller);

        caller_tree_node_builder.add_call(caller_id, callee_id, allocation.clone()).unwrap();

        call_graph.edge_entry(caller_id, callee_id)
            .or_insert(vec![])
            .push(allocation);
    }

    let caller_tree_node = caller_tree_node_builder.build();

    Ok((caller_tree_node, call_graph))
}

#[derive(Debug, Clone, PartialEq)]
pub enum Call {
    Inner(String),
    Leaf,
    Root,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Allocation {
    pub bytes: usize,
    pub location: Location,
}

impl Allocation {
    fn new(bytes: usize, location: Location) -> Self {
        Allocation { bytes, location }
    }
}

#[derive(Debug, Clone, PartialEq)]
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
