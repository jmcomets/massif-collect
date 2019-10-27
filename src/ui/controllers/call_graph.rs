use petgraph::prelude::*;

use crate::{
    CallGraph,
    CallId,
};

use super::CallListController;

pub struct CallGraphController<'a> {
    call_graph: &'a CallGraph,
    callers: CallListController,
    callees: CallListController,
    callees_selected: bool,
    history: Vec<CallId>,
}

impl<'a> CallGraphController<'a> {
    pub fn new(call_graph: &'a CallGraph) -> Self {
        let mut nav = CallGraphController {
            call_graph,
            callees_selected: true,
            callers: CallListController::empty(),
            callees: CallListController::empty(),
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

    pub fn callee_list(&self) -> (&CallListController, bool) {
        (&self.callees, self.callees_selected)
    }

    pub fn caller_list(&self) -> (&CallListController, bool) {
        (&self.callers, !self.callees_selected)
    }

    pub fn select_callees(&mut self) {
        self.callees_selected = true;
    }

    pub fn are_callees_selected(&self) -> bool {
        self.callees_selected
    }

    pub fn select_callers(&mut self) {
        self.callees_selected = false;
    }

    pub fn are_callers_selected(&self) -> bool {
        !self.callees_selected
    }

    fn current(&self) -> &CallListController {
        if self.callees_selected {
            &self.callees
        } else {
            &self.callers
        }
    }

    fn current_mut(&mut self) -> &mut CallListController {
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
        let callers = self.call_graph.neighbors_directed(call_id, Incoming)
            .map(|other_call_id| {
                let (caller_id, callee_id) = (other_call_id, call_id);
                let allocations = self.call_graph.edge_weight(caller_id, callee_id).unwrap();
                (caller_id, callee_id, allocations)
            });
        self.callers = CallListController::new(callers);

        let callees = self.call_graph.neighbors_directed(call_id, Outgoing)
            .map(|other_call_id| {
                let (caller_id, callee_id) = (call_id, other_call_id);
                let allocations = self.call_graph.edge_weight(caller_id, callee_id).unwrap();
                (caller_id, callee_id, allocations)
            });
        self.callees = CallListController::new(callees);
    }

    fn navigate_to_root(&mut self) {
        self.callers = CallListController::empty();

        let callees = self.call_graph.nodes()
            .filter(|&node| self.call_graph.neighbors_directed(node, Incoming).next().is_none())
            .flat_map(|node| self.call_graph.edges(node))
            .map(|(caller_id, callee_id, allocations)| (caller_id, callee_id, allocations));
        self.callees = CallListController::new(callees);
    }
}
