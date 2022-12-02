#![no_main]
use libfuzzer_sys::fuzz_target;
use sokoban::HashTable;
use sokoban_fuzz::{perform_action, NodeAllocatorMapAction};

fuzz_target!(|actions: Vec<NodeAllocatorMapAction::<u64, u64>>| {
    // fuzzed code goes here
    let mut tree = HashTable::<u64, u64, 2048, 8192>::default();
    let mut keys = Vec::new();
    for action in actions {
        perform_action(&mut tree, &mut keys, action);
    }
});
