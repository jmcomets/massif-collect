use std::{
    collections::HashMap,
    ops::{Deref, DerefMut},
};

use petgraph::graphmap::DiGraphMap;

use crate::{Address, Call, CallerTree};

pub type CallId = usize;

type CallGraphImpl = DiGraphMap<CallId, Vec<usize>>;

pub struct CallGraph {
    graph: CallGraphImpl,
    index: CallIndex,
}

impl CallGraph {
    pub fn from_tree(tree: &CallerTree) -> CallGraph {
        let mut index = CallIndex::new();
        let mut graph = CallGraphImpl::new();

        for (callee, caller, bytes) in tree.walk() {
            let callee_id = index.insert(callee);
            let caller_id = index.insert(caller);

            graph.edge_entry(caller_id, callee_id)
                .or_insert(vec![])
                .push(bytes);
        }

        CallGraph { graph, index }
    }

    pub fn get_call(&self, call_id: CallId) -> Option<&Call> {
        self.index.get(call_id)
    }
}

impl Deref for CallGraph {
    type Target = CallGraphImpl;

    fn deref(&self) -> &Self::Target {
        &self.graph
    }
}

impl DerefMut for CallGraph {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.graph
    }
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
    call_ids: HashMap<CallIndexKey, CallId>,
    calls: Vec<Call>,
}

impl CallIndex {
    pub fn new() -> Self {
        CallIndex {
            call_ids: HashMap::new(),
            calls: Vec::new(),
        }
    }

    pub fn insert(&mut self, call: &Call) -> CallId {
        let key = CallIndexKey::from(call);

        let ref mut calls = self.calls;
        let call_id = *self.call_ids.entry(key)
            .or_insert_with(|| {
                let call_id = calls.len();
                calls.push(call.clone());
                call_id
            });

        // TODO since we're lazy-inserting, make sure all the calls with the same key are the same

        call_id
    }

    pub fn get(&self, call_id: CallId) -> Option<&Call> {
        self.calls.get(call_id)
    }
}
