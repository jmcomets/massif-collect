use std::collections::HashMap;

pub type CallId = usize;
pub type Address = String; // TODO optimize storage of strings

#[derive(Debug)]
pub(crate) struct AddressIndex {
    max_call_id: CallId,
    call_ids: HashMap<Address, CallId>,
    call_addresses: Vec<Option<Address>>,
}

impl AddressIndex {
    pub fn new() -> Self {
        AddressIndex {
            max_call_id: 0,
            call_ids: HashMap::new(),
            call_addresses: Vec::new(),
        }
    }

    pub fn index(&mut self, address: Address) -> CallId {
        let ref mut max_call_id = self.max_call_id;
        let ref mut call_addresses = self.call_addresses;

        let call_id = *self.call_ids.entry(address.clone())
            .or_insert_with(|| {
                let call_id = *max_call_id;

                *max_call_id += 1;

                while call_id >= call_addresses.len() {
                    call_addresses.push(None);
                }

                call_id
            });

        self.call_addresses[call_id] = Some(address);

        call_id
    }

    pub fn address(&self, call_id: CallId) -> &str {
        self.call_addresses[call_id].as_ref().unwrap()
    }

    pub fn make_storage<T: Clone>(&self, default: T) -> Vec<T> {
        vec![default; self.max_call_id]
    }
}
