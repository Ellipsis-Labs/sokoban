#![no_main]
use libfuzzer_sys::fuzz_target;
use sokoban::DynamicRedBlackTree;
use sokoban_fuzz::{perform_action, NodeAllocatorMapAction};

fuzz_target!(|actions: Vec<NodeAllocatorMapAction::<u64, u64>>| {
    // fuzzed code goes here
    type RedBlackTree<'a> = DynamicRedBlackTree<'a, u64, u64>;
    let mut header_buf = vec![0u8; RedBlackTree::size_of_header()];
    let mut arena_bufs = vec![vec![0u8; RedBlackTree::size_of_nodes(1024)]; 8];
    let mut arena_slices: Vec<&mut [u8]> =
        arena_bufs.iter_mut().map(|a| a.as_mut_slice()).collect();
    let mut tree = RedBlackTree::new_from_buffers(
        header_buf.as_mut_slice(),
        arena_slices.as_mut_slice(),
        8,
        8192,
        1024,
    );
    let mut keys = Vec::new();
    for action in actions {
        perform_action(&mut tree, &mut keys, action);
        assert!(tree.is_valid_red_black_tree());
    }
});
