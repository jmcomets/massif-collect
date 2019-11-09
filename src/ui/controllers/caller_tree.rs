use std::collections::{LinkedList, HashSet};
use crate::{CallId, CallerTree, CallerTreeNode, Allocation};

type Item<'a> = (CallId, &'a CallerTreeNode, &'a Allocation, usize, f64);

pub struct CallerTreeController<'a> {
    tree: &'a CallerTree,
    skipped: LinkedList<Item<'a>>,
    before_selected: LinkedList<Item<'a>>,
    after_selected: LinkedList<Item<'a>>,
    expanded: HashSet<CallId>, // TODO replace with a bitset
}

impl<'a> CallerTreeController<'a> {
    pub fn new(tree: &'a CallerTree) -> Self {
        let mut controller = CallerTreeController {
            tree,
            skipped: <_>::default(),
            before_selected: <_>::default(),
            after_selected: <_>::default(),
            expanded: <_>::default(),
        };

        controller.reset();

        controller
    }

    pub fn reset(&mut self) {
        self.skipped.clear();
        self.before_selected.clear();
        self.after_selected.clear();
        self.expanded.clear();

        for (_, node) in self.tree.iter().rev() {
            self.expand_node(node, 0);
        }
    }

    pub fn toggle_selected(&mut self) {
        if let Some(selected_id) = self.after_selected.front().map(|(id, _, _, _, _)| *id) {
            let selected = self.after_selected.pop_front().unwrap();
            let (_, ref node, _, depth, _) = selected;

            let selected_is_expanded = self.expanded.contains(&selected_id);
            if selected_is_expanded {
                self.fold_node(node);
                self.expanded.remove(&selected_id);
            } else {
                self.expanded.insert(selected_id);
                self.expand_node(node, depth+1);
            }

            self.after_selected.push_front(selected);
        }
    }

    fn fold_node(&mut self, node: &'a CallerTreeNode) {
        for (id, ref node, _) in node.iter() {
            self.after_selected.pop_front().unwrap();
            if self.expanded.contains(&id) {
                self.fold_node(node);
            }
        }
    }

    fn expand_node(&mut self, selected_node: &'a CallerTreeNode, depth: usize) {
        let total_allocation: usize = selected_node.iter()
            .map(|(_, _, allocation)| allocation.bytes)
            .sum();
        let total_allocation = total_allocation as f64;

        for (id, node, allocation) in selected_node.iter().rev() {
            if self.expanded.contains(&id) {
                self.expand_node(node, depth);
            }
            let ratio = allocation.bytes as f64 / total_allocation;
            self.after_selected.push_front((id, node, allocation, depth, ratio));
        }
    }

    pub fn select_next(&mut self, limit: usize) {
        self.select_nth_next(1, limit)
    }

    pub fn select_previous(&mut self) {
        self.select_nth_previous(1)
    }

    pub fn select_nth_next(&mut self, n: usize, limit: usize) {
        for _ in 0..n {
            if self.after_selected.len() <= 1 {
                break;
            }

            let item = self.after_selected.pop_front().unwrap();
            self.before_selected.push_back(item);
        }

        for _ in limit..self.before_selected.len()+1 {
            let item = self.before_selected.pop_front().unwrap();
            self.skipped.push_back(item);
        }
    }

    pub fn select_nth_previous(&mut self, n: usize) {
        for _ in 0..n {
            if let Some(item) = self.before_selected.pop_back().or_else(|| self.skipped.pop_back()) {
                self.after_selected.push_front(item);
            } else {
                break;
            }
        }
    }

    pub fn is_selected(&self, index: usize) -> bool {
        self.before_selected.len() == index
    }

    pub fn iter(&self) -> impl Iterator<Item=(CallId, usize, &Allocation, usize, f64)> + '_ {
        macro_rules! transform_item {
            () => {{
                |&(id, node, allocation, depth, ratio)| {
                    (id, node.len(), allocation, depth, ratio)
                }
            }}
        }

        let before = self.before_selected.iter().map(transform_item!());
        let after = self.after_selected.iter().map(transform_item!());

        before.chain(after)
    }
}
