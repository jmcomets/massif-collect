use std::env;
use std::fs::File;
use std::io::{self, BufReader};

#[macro_use]
extern crate nom;

use petgraph::prelude::*;
use petgraph::visit::{depth_first_search, DfsEvent, Control};

mod indexing;
mod parsing;

use indexing::{AddressIndex, CallId};
use parsing::{Allocation, Call};

type CallGraph = DiGraphMap<CallId, Allocation>;

fn main() -> io::Result<()> {
    let mut address_index = AddressIndex::new();
    let mut call_graph = CallGraph::new();

    let filename = env::args().nth(1).unwrap_or("data/example.out".to_string());
    let file = File::open(filename)?;
    let reader = BufReader::new(file);

    for entry in parsing::massif_tree(reader) {
        let (caller, callee, allocation) = entry?;

        fn call_address(call: Call) -> String {
            match call {
                Call::Inner(address) => address,
                Call::Root           => "ROOT".to_string(),
                Call::Leaf           => "LEAF".to_string(),
            }
        }

        // leaf calls should always allocate through the system
        debug_assert!(!caller.is_leaf() || callee.is_none(), "{:?} -> {:?} : {:?}", caller, callee, allocation);

        let caller_id = address_index.index(call_address(caller));
        let callee_id = address_index.index(callee.map(call_address).unwrap_or("SYSTEM".to_string()));

        call_graph.add_edge(caller_id, callee_id, allocation);
    }

    // FIXME the allocations of the callers should sum up to the allocation of the callee
    #[cfg(debug_assertions)] {
        for callee1_id in call_graph.nodes() {
            for caller1_id in call_graph.neighbors_directed(callee1_id, Incoming) {
                let allocation1 = call_graph.edge_weight(caller1_id, callee1_id).unwrap();

                let callee2_id = caller1_id;
                if call_graph.neighbors_directed(callee2_id, Incoming).next().is_none() {
                    continue;
                }

                let alloc1 = allocation1.bytes;

                let mut alloc2 = 0;
                for caller2_id in call_graph.neighbors_directed(callee2_id, Incoming) {
                    let allocation2 = call_graph.edge_weight(caller2_id, callee2_id).unwrap();
                    alloc2 += allocation2.bytes;
                }

                assert!(alloc2 <= alloc1, "* -> {} ; {} -> {} ({} vs {})", caller1_id, caller1_id, callee1_id, alloc2, alloc1);

                let untracked = alloc1 - alloc2;
                if untracked > 0 {
                    println!("untracked alloc of {} bytes in {} -> {}", untracked, caller1_id, callee1_id);
                }
            }
        }
    }

    let mut call_depths = address_index.make_storage(0);

    for root_call_id in call_graph.nodes() {
        // only consider roots
        if call_graph.neighbors_directed(root_call_id, Incoming).next().is_some() {
            continue;
        }

        // TODO DFS on the root to sum all leaf allocations

        depth_first_search(&call_graph, Some(root_call_id), |event| {
            use DfsEvent::*;
            match event {
                TreeEdge(caller_id, callee_id) | BackEdge(caller_id, callee_id) | CrossForwardEdge(caller_id, callee_id) => {
                    let allocation = call_graph.edge_weight(caller_id, callee_id).unwrap();
                    let bytes = allocation.bytes;

                    // since the edges already aggregate the bytes allocated through call, this needn't be a DFS
                    // TODO cache this sum?
                    let mut total_call_bytes = 0;
                    for (_, _, allocation) in call_graph.edges(caller_id) {
                        total_call_bytes += allocation.bytes;
                    }

                    let call_ratio = 100. * (bytes as f64) / (total_call_bytes as f64);
                    // let root_ratio = 100. * (bytes as f64) / (total_root_bytes as f64);

                    let depth = call_depths[caller_id];
                    let address = address_index.address(callee_id);

                    for _ in 0..depth { print!(" "); }
                    print!("{}", bytes);
                    // print!(" (");
                    if bytes < total_call_bytes {
                        print!(" (");
                        print!("{:.2}% of {} [call]", call_ratio, total_call_bytes);
                        // print!(", ");
                        print!(")");
                    }
                    // print!("{:.2}% of {} [total]", root_ratio, total_root_bytes);
                    // print!(")");
                    print!(": {} {}", address, allocation.details.to_string());
                    println!();

                    call_depths[callee_id] = depth + 1;
                }

                _ => {}
            }

            Control::<()>::Continue
        });
    }

    Ok(())
}
