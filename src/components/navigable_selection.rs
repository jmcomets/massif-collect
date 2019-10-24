use std::cmp;

pub struct NavigableSelection<T: AsRef<str>> {
    items: Vec<T>,
    selected: Option<usize>,
}

impl<T: AsRef<str>> NavigableSelection<T> {
    pub fn new(items: Vec<T>) -> Self {
        let selected = if !items.is_empty() { Some(0) } else { None };
        NavigableSelection { items, selected }
    }

    pub fn select_first(&mut self) {
        if let Some(selected) = self.selected.as_mut() {
            *selected = 0;
        }
    }

    pub fn select_last(&mut self) {
        if let Some(selected) = self.selected.as_mut() {
            *selected = self.items.len() - 1;
        }
    }

    pub fn select_next(&mut self) {
        self.select_nth_next(1)
    }

    pub fn select_previous(&mut self) {
        self.select_nth_previous(1)
    }

    pub fn select_nth_next(&mut self, n: usize) {
        if let Some(i) = self.selected.as_mut() {
            *i += cmp::min(self.items.len()-1-*i, n);
        }
    }

    pub fn select_nth_previous(&mut self, n: usize) {
        if let Some(i) = self.selected.as_mut() {
            *i -= cmp::min(*i, n);
        }
    }

    pub fn items(&self) -> &[T] {
        &self.items[..]
    }

    pub fn selected_index(&self) -> Option<usize> {
        self.selected
    }

    pub fn selected_item(&self) -> Option<&T> {
        self.selected.as_ref()
            .map(|&i| &self.items[i])
    }
}
