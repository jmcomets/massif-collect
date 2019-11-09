use crate::{CallId, CallerTree};

type Node<'a> = (Option<CallId>, &'a CallerTree);

type CallerTreeStack<'a> = Vec<(Node<'a>, usize)>;

pub struct CallerTreeController<'a> {
    stack: CallerTreeStack<'a>,
}

impl<'a> CallerTreeController<'a> {
    pub fn new(root: &'a CallerTree) -> Self {
        CallerTreeController { stack: vec![((None, root), 0)] }
    }

    pub fn select_first(&mut self) {
        unimplemented!()
    }

    pub fn select_last(&mut self) {
        unimplemented!()
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

    pub fn iter(&self) -> impl Iterator<Item=(CallId, usize)> + '_ {
        self.stack.iter().enumerate().rev()
            .flat_map(|(depth, &((_, tree), skip))| {
                tree.iter().skip(skip).map(move |(caller_id, _)| (caller_id, depth))
            })
    }
}
