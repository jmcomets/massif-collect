use crate::CallerTree;

pub struct CallerTreeController<'a> {
    caller_tree: &'a CallerTree,
}

impl<'a> CallerTreeController<'a> {
    pub fn new(caller_tree: &'a CallerTree) -> Self {
        CallerTreeController { caller_tree }
    }
}
