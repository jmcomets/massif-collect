use std::collections::HashMap;

#[macro_use]
extern crate nom;

pub mod ui;

pub mod graph;
pub mod parsing;

pub type SnapshotId = usize;

pub type Attributes = HashMap<String, String>;

#[derive(Debug, PartialEq)]
pub struct Snapshot {
    id: SnapshotId,
    attributes: Attributes,
    tree: CallerTree,
}

pub type Address = usize;

#[derive(Debug, PartialEq)]
pub enum Call {
    Sampled(Option<Address>, String),
    Ignored(usize, f32),
}

impl ToString for Call {
    fn to_string(&self) -> String {
        match self {
            Call::Sampled(_, description)   => description.clone(),
            Call::Ignored(count, threshold) => {
                let plural = if count > &1 { "s" } else { "" };
                format!("in {} place{}, below massif's threshold ({:.2}%)", count, plural, threshold)
            }
        }
    }
}

#[derive(Debug, PartialEq)]
pub struct Sample {
    pub bytes: usize,
    pub call: Call,
}

#[derive(Debug, PartialEq)]
pub struct CallerTree {
    pub sample: Sample,
    pub callers: Vec<CallerTree>,
}

impl CallerTree {
    pub fn walk(&self) -> CallerTreeWalker {
        CallerTreeWalker { stack: vec![(&self.sample, &self.callers[..])] }
    }
}

pub struct CallerTreeWalker<'a> {
    stack: Vec<(&'a Sample, &'a [CallerTree])>,
}

impl<'a> Iterator for CallerTreeWalker<'a> {
    type Item = (&'a Call, &'a Call, usize);

    fn next(&mut self) -> Option<Self::Item> {
        while let Some((sample, callers)) = self.stack.last_mut() {
            let ref callee = sample.call;
            let bytes = sample.bytes;

            if let Some((caller_tree, rest)) = callers.split_first() {
                *callers = rest;

                let ref caller = caller_tree.sample.call;
                self.stack.push((&caller_tree.sample, &caller_tree.callers));
                return Some((callee, caller, bytes));
            } else {
                self.stack.pop();
            }
        }

        None
    }
}
