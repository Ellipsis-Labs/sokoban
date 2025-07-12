#![no_main]
use libfuzzer_sys::fuzz_target;
use sokoban::StaticRedBlackTree;
use sokoban_fuzz::{perform_action, NodeAllocatorMapAction};

fuzz_target!(|actions: Vec<NodeAllocatorMapAction::<u64, u64>>| {
    // fuzzed code goes here
    type RedBlackTree<'a> = StaticRedBlackTree<'a, u64, u64, 8192>;
    let mut buf = vec![0u8; RedBlackTree::size_of_buffer()];
    let mut tree = RedBlackTree::new_from_buffer(buf.as_mut_slice());
    let mut keys = Vec::new();
    for action in actions {
        perform_action(&mut tree, &mut keys, action);
        assert!(tree.is_valid_red_black_tree());
    }
});
