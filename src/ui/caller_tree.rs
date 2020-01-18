use std::{
    collections::{LinkedList, HashSet},
    fmt::Write,
};

use tui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Modifier, Style},
    widgets::Widget,
};
use crate::{graph::CallId, tree::CallerTree};

pub struct CallerTreeWidget<'a>(&'a CallerTreeController<'a>);

impl<'a> CallerTreeWidget<'a> {
    pub fn new(controller: &'a CallerTreeController<'a>) -> Self {
        CallerTreeWidget(controller)
    }
}

macro_rules! color_gradient {
    ($x:expr, $color1:expr, $color2:expr) => {{
        let (r1, g1, b1) = $color1;
        let (r2, g2, b2) = $color2;
        (
            (r1 as f64 + $x * (r2 as f64 - r1 as f64)) as u8,
            (g1 as f64 + $x * (g2 as f64 - g1 as f64)) as u8,
            (b1 as f64 + $x * (b2 as f64 - b1 as f64)) as u8,
        )
    }}
}

impl<'a> Widget for CallerTreeWidget<'a> {
    fn draw(&mut self, area: Rect, buf: &mut Buffer) {
        for (i, (_, nb_callers, allocation, depth, ratio)) in self.0.iter().enumerate().take(area.height as usize) {
            let indent = depth * 2;
            if (indent as u16) < area.right() {
                let (x, y) = (area.left(), area.top() + i as u16);

                // pad the line to include the indent & at least the area's width
                let mut line = String::with_capacity(area.width as usize);
                for _ in 0..indent { line.push(' '); }
                write!(&mut line, "n{}: {} {}", nb_callers, allocation.bytes, allocation.location.to_string()).unwrap();
                for _ in line.len()..area.width as usize { line.push(' '); }

                let mut style = Style::default().fg(Color::Black);

                // highlight the selected item
                if self.0.is_selected(i) {
                    style = style.modifier(Modifier::BOLD);
                }

                // map the allocation ratio to a color
                debug_assert!(ratio <= 1.);
                let (red, green, blue) = color_gradient!(ratio, (0xf5, 0xaf, 0x19), (0xf1, 0x27, 0x11));
                style = style.bg(Color::Rgb(red, green, blue));

                buf.set_string(x, y, line, style);
            }
        }
    }
}

// old caller tree controller

type CallerTreeItem<'a> = (CallId, &'a CallerTree, &'a Allocation, usize, f64);

pub struct CallerTreeController<'a> {
    tree: &'a CallerTree,
    skipped: LinkedList<CallerTreeItem<'a>>,
    before_selected: LinkedList<CallerTreeItem<'a>>,
    after_selected: LinkedList<CallerTreeItem<'a>>,
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

    fn fold_node(&mut self, node: &'a CallerTree) {
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
