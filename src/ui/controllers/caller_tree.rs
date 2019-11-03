use crate::{CallId, CallerTree, iters};

pub struct CallerTreeController<'a> {
    root: &'a CallerTree,
    skip: usize,
    stack: Vec<(CallId, &'a CallerTree, usize)>,
}

impl<'a> CallerTreeController<'a> {
    pub fn new(root: &'a CallerTree) -> Self {
        CallerTreeController { root, skip: 0, stack: vec![] }
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

    pub fn iter(&self) -> impl Iterator<Item=(Option<CallId>, &'a CallerTree, usize)> + '_ {
        macro_rules! lift {
            ($it:expr) => {{
                $it.map(|(caller_id, caller, depth)| (Some(caller_id), caller, depth))
            }}
        }

        let stack_it = lift!((1..self.stack.len()).zip(self.stack.iter()).rev()
            .flat_map(|(depth, &(caller_id, caller, i))| {
                iters::prefixed((caller_id, caller, depth), caller.walk_callers().skip(i))
            }));

        let root_it = iters::prefixed((None, self.root, 0), lift!(self.root.walk_callers().skip(self.skip)));

        stack_it.chain(root_it)
    }
}

