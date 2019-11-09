use crate::{CallId, CallerTree, CallerTreeNode, Allocation};

type CallerTreeStack<'a> = Vec<((Option<CallId>, &'a CallerTreeNode), usize)>;

pub struct CallerTreeController<'a> {
    tree: &'a CallerTree,
    skip: usize,
    stack: CallerTreeStack<'a>,
}

impl<'a> CallerTreeController<'a> {
    pub fn new(tree: &'a CallerTree) -> Self {
        CallerTreeController { tree, skip: 0, stack: vec![] }
    }

    pub fn reset(&mut self) {
        self.skip = 0;
        self.stack.clear();
    }

    pub fn select_next(&mut self) {
        unimplemented!()
    }

    pub fn select_previous(&mut self) {
        unimplemented!()
    }

    pub fn select_nth_next(&mut self, _n: usize) {
        unimplemented!()
    }

    pub fn select_nth_previous(&mut self, _n: usize) {
        unimplemented!()
    }

    pub fn iter(&self) -> impl Iterator<Item=(CallId, &Allocation, usize)> + '_ {
        if !self.stack.is_empty() {
            self.stack.iter().enumerate().rev()
                .flat_map(|(depth, &((_, subtree), skip))| {
                    subtree.iter().skip(skip)
                            .map(move |(id, _, allocation)| (id, allocation, depth))
                })
        } else {
        }
    }
}
