#![no_main]
use libfuzzer_sys::fuzz_target;
use sokoban::Critbit;
use sokoban_fuzz::{perform_action, NodeAllocatorMapAction};

fuzz_target!(|actions: Vec<NodeAllocatorMapAction::<u128, u64>>| {
    // fuzzed code goes here
    let mut tree = Critbit::<u64, 2048, 1024>::default();
    let mut keys = Vec::new();
    for action in actions {
        println!("{:?}", action);
        perform_action(&mut tree, &mut keys, action);
    }
});
