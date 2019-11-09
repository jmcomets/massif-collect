use std::collections::{LinkedList, HashSet};
use crate::{CallId, CallerTree, CallerTreeNode, Allocation};

type Item<'a> = (CallId, &'a CallerTreeNode, Option<&'a Allocation>, usize);

pub struct CallerTreeController<'a> {
    tree: &'a CallerTree,
    before_selected: LinkedList<Item<'a>>,
    after_selected: LinkedList<Item<'a>>,
    expanded: HashSet<CallId>, // TODO replace with a bitset
}

impl<'a> CallerTreeController<'a> {
    pub fn new(tree: &'a CallerTree) -> Self {
        let mut controller = CallerTreeController {
            tree,
            before_selected: <_>::default(),
            after_selected: <_>::default(),
            expanded: <_>::default(),
        };

        controller.reset();

        controller
    }

    pub fn reset(&mut self) {
        self.before_selected.clear();
        self.after_selected.clear();
        self.after_selected.extend(self.tree.iter().map(|(id, node)| (*id, node, None, 0)));
        self.expanded.clear();
    }

    pub fn toggle_selected(&mut self) {
        if let Some(selected_id) = self.after_selected.front().map(|(id, _, _, _)| *id) {
            let selected = self.after_selected.pop_front().unwrap();
            let (_, ref node, _, depth) = selected;

            let selected_is_expanded = self.expanded.contains(&selected_id);
            if selected_is_expanded {
                self.fold_node(node);
                self.expanded.remove(&selected_id);
            } else {
                self.expand_node(node, depth + 1);
                self.expanded.insert(selected_id);
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
        for (id, node, allocation) in selected_node.iter().rev() {
            if self.expanded.contains(&id) {
                self.expand_node(node, depth + 1);
            }
            self.after_selected.push_front((id, node, Some(allocation), depth));
        }
    }


    pub fn select_next(&mut self) {
        self.select_nth_next(1)
    }

    pub fn select_previous(&mut self) {
        self.select_nth_previous(1)
    }

    pub fn select_nth_next(&mut self, mut n: usize) {
        loop {
            if n == 0 {
                break;
            }
            n -= 1;

            if self.after_selected.len() == 1 {
                break;
            }

            if let Some(item) = self.after_selected.pop_front() {
                self.before_selected.push_back(item);
            } else {
                break;
            }
        }
    }

    pub fn select_nth_previous(&mut self, mut n: usize) {
        loop {
            if n == 0 {
                break;
            }
            n -= 1;

            if let Some(item) = self.before_selected.pop_back() {
                self.after_selected.push_front(item);
            } else {
                break;
            }
        }
    }

    pub fn is_selected(&self, index: usize) -> bool {
        self.before_selected.len() == index
    }

    pub fn iter(&self) -> impl Iterator<Item=(CallId, usize, Option<&Allocation>, usize)> + '_ {
        self.before_selected.iter().map(|&(id, node, allocation, depth)| (id, node.len(), allocation, depth)).chain(
            self.after_selected.iter().map(|&(id, node, allocation, depth)| (id, node.len(), allocation, depth))
        )
    }
}
