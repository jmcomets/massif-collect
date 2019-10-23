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
        if let Some(selected) = self.selected.as_mut() {
            if *selected+1 < self.items.len() {
                *selected += 1;
            }
        }
    }

    pub fn select_previous(&mut self) {
        if let Some(selected) = self.selected.as_mut() {
            if *selected > 0 {
                *selected -= 1;
            }
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
