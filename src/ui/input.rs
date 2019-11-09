use tui::{
    layout::Rect,
};

use super::{
    controllers::{
        CallGraphController,
        CallerTreeController,
    },
    events::Key,
};

pub trait InputHandler {
    fn handle_input(&mut self, area: Rect, input: &Key);
}

impl<'a> InputHandler for CallGraphController<'a> {
    fn handle_input(&mut self, area: Rect, input: &Key) {
        let page_height = area.height as usize;
        match input {
            Key::Down | Key::Char('j') => { self.select_next(); }
            Key::Up | Key::Char('k')   => { self.select_previous(); }
            Key::Home                  => { self.select_first(); }
            Key::End | Key::Char('G')  => { self.select_last(); }

            Key::PageDown | Key::Char('f') => { self.select_nth_next(page_height); }
            Key::PageUp | Key::Char('b') => { self.select_nth_previous(page_height); }

            Key::Left | Key::Char('h')  => {
                if !self.are_callers_selected()
                {
                    self.select_callers();
                }
                else
                {
                    self.enter_selected();
                }
            }
            Key::Right | Key::Char('l') => {
                if !self.are_callees_selected()
                {
                    self.select_callees();
                }
                else
                {
                    self.enter_selected();
                }
            }

            Key::Char('\n') => { self.enter_selected(); }
            Key::Backspace  => { self.leave_current(); }

            _ => {}
        }
    }
}

impl<'a> InputHandler for CallerTreeController<'a> {
    fn handle_input(&mut self, area: Rect, input: &Key) {
        let page_height = area.height as usize;
        match input {
            Key::Down | Key::Char('j') => { self.select_next(page_height); }
            Key::Up | Key::Char('k')   => { self.select_previous(); }
            Key::Home                  => { self.reset(); }
            Key::Char('\n')            => { self.toggle_selected(); }

            Key::PageDown | Key::Char('f') => { self.select_nth_next(page_height, page_height); }
            Key::PageUp | Key::Char('b') => { self.select_nth_previous(page_height); }

            _ => {}
        }
    }
}
