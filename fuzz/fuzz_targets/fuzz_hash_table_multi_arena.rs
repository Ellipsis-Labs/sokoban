#![no_main]
use libfuzzer_sys::fuzz_target;
use sokoban::DynamicHashTable;
use sokoban_fuzz::{perform_action, NodeAllocatorMapAction};

fuzz_target!(|actions: Vec<NodeAllocatorMapAction::<u64, u64>>| {
    // fuzzed code goes here
    type HashTable<'a> = DynamicHashTable<'a, u64, u64, 2048>;
    let mut header_buf = vec![0u8; HashTable::size_of_header()];
    let mut arena_bufs = vec![vec![0u8; HashTable::size_of_nodes(4096)]; 2];
    let mut arena_slices: Vec<&mut [u8]> =
        arena_bufs.iter_mut().map(|a| a.as_mut_slice()).collect();
    let mut tree = HashTable::new_from_buffers(
        header_buf.as_mut_slice(),
        arena_slices.as_mut_slice(),
        2,
        8192,
        4096,
    );
    let mut keys = Vec::new();
    for action in actions {
        perform_action(&mut tree, &mut keys, action);
    }
});
