use std::time::Instant;

use sokoban::{NodeAllocatorMap, RedBlackTree};

#[derive(Default, PartialEq, PartialOrd, Clone, Copy, Debug, Ord, Eq)]
#[repr(C)]
/// Price-time priority key
struct Key {
    price: u64,
    id: u64,
}
impl Key {
    fn rand() -> Self {
        Self {
            price: rand::random(),
            id: rand::random(),
        }
    }
}
unsafe impl bytemuck::Pod for Key {}
unsafe impl bytemuck::Zeroable for Key {}

#[derive(Default, PartialEq, PartialOrd, Clone, Copy)]
#[repr(C)]
/// Mock limit order key
struct Entry {
    lots: u64,
    maker: u32,
    _pad: [u8; 4],
}

impl Entry {
    fn rand() -> Self {
        Entry {
            lots: 10,
            maker: 0,
            _pad: [0; 4],
        }
    }
    fn rand_with_maker(idx: u32) -> Self {
        assert!(idx > 0); // 0 is reserved
        Entry {
            lots: 10,
            maker: idx,
            _pad: [0; 4],
        }
    }
}

unsafe impl bytemuck::Pod for Entry {}
unsafe impl bytemuck::Zeroable for Entry {}

fn main() {
    const ITERS: usize = 1000;
    const WARMUP_ITERS: usize = 100;

    const TARGET_MAKER: u32 = 5;

    const TREE_SIZE: usize = 4096;
    const REMOVE: usize = 256;

    let mut total_remove_micros = 0;
    for i in 0..ITERS + WARMUP_ITERS {
        // Setup
        let mut tree = RedBlackTree::<Key, Entry, TREE_SIZE>::new();
        for i in 0..TREE_SIZE {
            if i < REMOVE {
                tree.insert(Key::rand(), Entry::rand_with_maker(TARGET_MAKER));
            } else {
                tree.insert(Key::rand(), Entry::rand());
            }
        }

        // Start filter
        let timer = Instant::now();
        let keys = tree
            .iter()
            .filter(|(_key, entry)| entry.maker == TARGET_MAKER)
            .map(|(key, _)| *key)
            .collect::<Vec<_>>();
        for key in keys {
            tree.remove(&key);
        }
        if i > WARMUP_ITERS {
            total_remove_micros += timer.elapsed().as_micros();
        }
        assert_eq!(tree.len(), TREE_SIZE - REMOVE);
    }
    println!("average id + remove: {total_remove_micros} micros");

    let mut total_drain_alloc_micros = 0;
    for i in 0..ITERS + WARMUP_ITERS {
        // Setup
        let mut tree = RedBlackTree::<Key, Entry, TREE_SIZE>::new();
        for i in 0..TREE_SIZE {
            if i < REMOVE {
                tree.insert(Key::rand(), Entry::rand_with_maker(TARGET_MAKER));
            } else {
                tree.insert(Key::rand(), Entry::rand());
            }
        }

        // Start filter
        let timer = Instant::now();
        drop(
            tree.drain_filter(
                #[inline(always)]
                |_k, v| v.maker == TARGET_MAKER,
            )
            .collect::<Vec<_>>(),
        );
        if i > WARMUP_ITERS {
            total_drain_alloc_micros += timer.elapsed().as_micros();
        }
        assert_eq!(tree.len(), TREE_SIZE - REMOVE);
    }
    println!("average drain_alloc: {total_drain_alloc_micros} micros");

    let mut total_drain_micros = 0;
    for i in 0..ITERS + WARMUP_ITERS {
        // Setup
        let mut tree = RedBlackTree::<Key, Entry, TREE_SIZE>::new();
        for i in 0..TREE_SIZE {
            if i < REMOVE {
                tree.insert(Key::rand(), Entry::rand_with_maker(TARGET_MAKER));
            } else {
                tree.insert(Key::rand(), Entry::rand());
            }
        }

        // Start filter
        let timer = Instant::now();
        for _x in tree.drain_filter(
            #[inline(always)]
            |_k, v| v.maker == TARGET_MAKER,
        ) {}
        if i > WARMUP_ITERS {
            total_drain_micros += timer.elapsed().as_micros();
        }
        assert_eq!(tree.len(), 4096 - REMOVE);
    }
    println!("average drain: {total_drain_micros} micros");
}
