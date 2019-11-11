use std::collections::HashMap;

use crate::{Address, Call, CallId};

#[derive(Debug, Eq, PartialEq, Hash)]
enum CallIndexKey {
    Root,
    Inner(Address),
    Leaf,
}

impl<'a> From<&'a Call> for CallIndexKey {
    fn from(call: &'a Call) -> Self {
        match call {
            Call::Sampled(None, _)          => Self::Leaf,
            Call::Sampled(Some(address), _) => Self::Inner(*address),
            Call::Ignored(_, _)             => Self::Root,
        }
    }
}

#[derive(Debug)]
pub struct CallIndex {
    max_call_id: CallId,
    call_ids: HashMap<CallIndexKey, CallId>,
}

impl CallIndex {
    pub fn new() -> Self {
        CallIndex {
            max_call_id: 0,
            call_ids: HashMap::new(),
        }
    }

    pub fn index(&mut self, call: &Call) -> CallId {
        let key = call.into();

        let ref mut max_call_id = self.max_call_id;

        let call_id = *self.call_ids.entry(key)
            .or_insert_with(|| {
                let call_id = *max_call_id;

                *max_call_id += 1;

                call_id
            });

        call_id
    }
}
