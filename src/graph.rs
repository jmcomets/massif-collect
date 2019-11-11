use std::collections::HashMap;

use petgraph::graphmap::DiGraphMap;

use crate::{Address, Call, CallerTree};

pub type CallId = usize;

pub type CallGraph = DiGraphMap<CallId, Vec<usize>>;

pub fn build_call_graph(caller_tree: &CallerTree) -> CallGraph {
    let mut call_index = CallIndex::new();
    let mut call_graph = CallGraph::new();

    for (callee, caller, bytes) in caller_tree.walk() {
        let callee_id = call_index.index(callee);
        let caller_id = call_index.index(caller);

        call_graph.edge_entry(caller_id, callee_id)
            .or_insert(vec![])
            .push(bytes);
     }

    call_graph
}

#[derive(Debug, Eq, PartialEq, Hash)]
enum CallIndexKey {
    Root,
    Node(Address),
    Leaf,
}

impl<'a> From<&'a Call> for CallIndexKey {
    fn from(call: &'a Call) -> Self {
        match call {
            Call::Sampled(None, _)          => Self::Leaf,
            Call::Sampled(Some(address), _) => Self::Node(*address),
            Call::Ignored(_, _)             => Self::Root, // TODO maybe assign different ids to each ignored sample?
        }
    }
}

#[derive(Debug)]
pub struct CallIndex {
    max_call_id: CallId,
    call_ids: HashMap<CallIndexKey, CallId>,
}

impl CallIndex {
    pub fn new() -> Self {
        CallIndex {
            max_call_id: 0,
            call_ids: HashMap::new(),
        }
    }

    pub fn index(&mut self, call: &Call) -> CallId {
        let key = call.into();

        let ref mut max_call_id = self.max_call_id;

        let call_id = *self.call_ids.entry(key)
            .or_insert_with(|| {
                let call_id = *max_call_id;

                *max_call_id += 1;

                call_id
            });

        call_id
    }
}
