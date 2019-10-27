use std::collections::HashSet;

use std::ops::{Deref, DerefMut};

use crate::{
    Allocation,
    CallId,
    Location,
};

use super::SelectionListController;

pub struct CallListController(SelectionListController<CallStack>);

impl CallListController {
    pub fn new<T, Allocs>(iter: T) -> Self
        where T: IntoIterator<Item=(CallId, CallId, Allocs)>,
              Allocs: AsRef<[Allocation]>
    {
        let mut stacks: Vec<_> = iter.into_iter()
            .map(|(caller_id, callee_id, allocations)| CallStack::new(caller_id, callee_id, allocations.as_ref()))
            .collect();
        stacks.sort_by_key(|stack| stack.allocated_bytes);
        stacks.reverse();
        CallListController(SelectionListController::new(stacks))
    }

    pub fn empty() -> Self {
        CallListController(SelectionListController::new(vec![]))
    }
}

impl Deref for CallListController {
    type Target = SelectionListController<CallStack>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for CallListController {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

pub struct CallStack {
    pub caller_id: CallId,
    pub callee_id: CallId,
    pub description: String,
    pub allocated_bytes: usize,
}

impl CallStack {
    fn new(caller_id: CallId, callee_id: CallId, allocations: &[Allocation]) -> Self {
        let location = merge_locations(allocations.iter().map(|alloc| &alloc.location)).unwrap();
        let bytes = allocations.iter().map(|alloc| alloc.bytes).sum();
        CallStack {
            caller_id, callee_id,
            description: format!("{} bytes {}", bytes, location.to_string()),
            allocated_bytes: bytes,
        }
    }
}

fn merge_locations<'a, T: IntoIterator<Item=&'a Location>>(iter: T) -> Option<Location> {
    let mut described = HashSet::new();
    let mut omitted = None;
    for location in iter {
        match location {
            Location::Described(description) => {
                described.insert(description);
            }

            Location::Omitted((count, threshold)) => {
                let (existing_count, existing_threshold) = omitted.get_or_insert((0, threshold));
                *existing_count += count;
                if threshold != *existing_threshold {
                    return None; // TODO throw error
                }
            }
        }
    }

    if !described.is_empty() && omitted.is_some() {
        return None; // TODO throw error
    }

    if described.is_empty() && omitted.is_none() {
        return None; // TODO throw error
    }

    if let Some((count, threshold)) = omitted {
        Some(Location::Omitted((count, *threshold)))
    } else {
        let mut merged_description = "".to_string();
        for description in described {
            if !merged_description.is_empty() {
                merged_description += " / ";
            }
            merged_description += description;
        }
        Some(Location::Described(merged_description))
    }
}

impl AsRef<str> for CallStack {
    fn as_ref(&self) -> &str {
        &self.description
    }
}
