use petgraph::prelude::*;

use crate::{
    Allocation,
    CallGraph,
    CallId,
    components::NavigableSelection,
};

pub struct CallGraphNavigator<'a> {
    call_graph: &'a CallGraph,
    callers: CallList,
    callees: CallList,
    callees_selected: bool,
    history: Vec<CallId>,
}

impl<'a> CallGraphNavigator<'a> {
    pub fn new(call_graph: &'a CallGraph) -> Self {
        let mut nav = CallGraphNavigator {
            call_graph,
            callees_selected: true,
            callers: CallList::new(vec![]),
            callees: CallList::new(vec![]),
            history: vec![],
        };

        nav.navigate_to_root();

        nav
    }

    pub fn select_first(&mut self) {
        self.current_mut().select_first();
    }

    pub fn select_last(&mut self) {
        self.current_mut().select_last();
    }

    pub fn select_next(&mut self) {
        self.current_mut().select_next();
    }

    pub fn select_previous(&mut self) {
        self.current_mut().select_previous();
    }

    pub fn select_nth_next(&mut self, n: usize) {
        self.current_mut().select_nth_next(n);
    }

    pub fn select_nth_previous(&mut self, n: usize) {
        self.current_mut().select_nth_previous(n);
    }

    pub fn callee_list(&self) -> (&CallList, bool) {
        (&self.callees, self.callees_selected)
    }

    pub fn caller_list(&self) -> (&CallList, bool) {
        (&self.callers, !self.callees_selected)
    }

    pub fn select_callees(&mut self) {
        self.callees_selected = true;
    }

    pub fn select_callers(&mut self) {
        self.callees_selected = false;
    }

    fn current(&self) -> &CallList {
        if self.callees_selected {
            &self.callees
        } else {
            &self.callers
        }
    }

    fn current_mut(&mut self) -> &mut CallList {
        if self.callees_selected {
            &mut self.callees
        } else {
            &mut self.callers
        }
    }

    pub fn enter_selected(&mut self) {
        if let Some(stack) = self.current().selected_item() {
            let call_id = if self.callees_selected { stack.callee_id } else { stack.caller_id };
            self.history.push(call_id);
            self.navigate_to(call_id);
        }
    }

    pub fn leave_current(&mut self) {
        self.history.pop();
        if let Some(&call_id) = self.history.last() {
            self.navigate_to(call_id);
        } else {
            self.navigate_to_root();
        }
    }

    fn navigate_to(&mut self, call_id: CallId) {
        self.callers = new_call_list(self.call_graph.neighbors_directed(call_id, Incoming)
            .map(|other_call_id| {
                let (caller_id, callee_id) = (other_call_id, call_id);
                let allocation = self.call_graph.edge_weight(caller_id, callee_id).unwrap();
                CallStack::new(caller_id, callee_id, allocation)
            }));

        self.callees = new_call_list(self.call_graph.neighbors_directed(call_id, Outgoing)
            .map(|other_call_id| {
                let (caller_id, callee_id) = (call_id, other_call_id);
                let allocation = self.call_graph.edge_weight(caller_id, callee_id).unwrap();
                CallStack::new(caller_id, callee_id, allocation)
            }));
    }

    fn navigate_to_root(&mut self) {
        self.callers = CallList::new(vec![]);

        let callees = self.call_graph.nodes()
            .filter(|&node| self.call_graph.neighbors_directed(node, Incoming).next().is_none())
            .flat_map(|node| self.call_graph.edges(node))
            .map(|(caller_id, callee_id, allocation)| CallStack::new(caller_id, callee_id, allocation));
        self.callees = new_call_list(callees);
    }
}

pub type CallList = NavigableSelection<CallStack>;

pub fn new_call_list<T: IntoIterator<Item=CallStack>>(iter: T) -> CallList {
    let mut stacks: Vec<_> = iter.into_iter().collect();
    stacks.sort_by_key(|stack| stack.allocated_bytes);
    stacks.reverse();
    CallList::new(stacks)
}

pub struct CallStack {
    caller_id: CallId,
    callee_id: CallId,
    description: String,
    allocated_bytes: usize,
}

impl CallStack {
    fn new(caller_id: CallId, callee_id: CallId, allocation: &Allocation) -> Self {
        CallStack {
            caller_id, callee_id,
            description: format!("{} bytes {}", allocation.bytes, allocation.location.to_string()),
            allocated_bytes: allocation.bytes,
        }
    }
}

impl AsRef<str> for CallStack {
    fn as_ref(&self) -> &str {
        &self.description
    }
}
