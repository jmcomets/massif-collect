use std::collections::HashMap;

use crate::Call;

pub type CallId = usize;

type Address = String; // TODO optimize storage of strings

#[derive(Debug)]
pub struct CallIndex {
    max_call_id: CallId,
    call_ids: HashMap<Address, CallId>,
}

impl CallIndex {
    pub fn new() -> Self {
        CallIndex {
            max_call_id: 0,
            call_ids: HashMap::new(),
        }
    }

    pub fn index(&mut self, call: Call) -> CallId {
        let address = match call {
            Call::Inner(address) => address,
            Call::Root           => "ROOT".to_string(),
            Call::Leaf           => "LEAF".to_string(),
        };

        self.get_or_insert_address(address)
    }

    pub fn index_leaf_caller(&mut self) -> CallId {
        self.get_or_insert_address("SYSTEM".to_string())
    }

    fn get_or_insert_address(&mut self, address: Address) -> CallId {
        let ref mut max_call_id = self.max_call_id;

        let call_id = *self.call_ids.entry(address)
            .or_insert_with(|| {
                let call_id = *max_call_id;

                *max_call_id += 1;

                call_id
            });

        call_id
    }
}
