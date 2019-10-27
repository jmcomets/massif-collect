use tui::{
    backend::Backend,
    terminal::Frame,
};

use crate::ui::controllers::CallerTreeController;

pub fn render_caller_tree<B: Backend>(_caller_tree: &CallerTreeController, _f: &mut Frame<B>) {
    // TODO
}
