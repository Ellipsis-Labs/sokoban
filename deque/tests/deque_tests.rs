use deque::*;
use node_allocator::*;
use rand::thread_rng;
use rand::{self, Rng};
use std::collections::VecDeque;


const MAX_SIZE: usize = 256;

#[tokio::test(threaded_scheduler)]
async fn test_initialize() {
    let dll = Deque::<u64, MAX_SIZE>::new();

    assert_eq!(
        dll.sequence_number, 0,
        "Init failed to set sequence properly"
    );

    assert_eq!(dll.head, SENTINEL, "Init failed to set head properly");

    assert_eq!(dll.tail, SENTINEL, "Init failed to set tail properly");
}

#[tokio::test(threaded_scheduler)]
async fn test_simple() {
    let mut dll = Deque::<u64, MAX_SIZE>::new();
    let mut dll_std = VecDeque::new();
    let mut rng = thread_rng();

    for _ in 0..MAX_SIZE - 1 {
        let v = rng.gen::<u64>();
        dll.push_back(v);
        dll_std.push_back(v);
        assert_eq!(
            dll.front().unwrap(),
            dll_std.front().unwrap(),
            "DLL front mismatch",
        );
        assert_eq!(
            dll.back().unwrap(),
            dll_std.back().unwrap(),
            "DLL back mismatch",
        );
    }

    assert_eq!(
        dll_std.len(),
        MAX_SIZE - 1_usize,
        "DLL (std) size is wrong"
    );
    assert_eq!(dll.len(), MAX_SIZE - 1_usize, "DLL size is wrong");

    for (i, (_p, node)) in dll.iter_mut().enumerate() {
        *node = i as u64;
    }
    for (i, (_p, node)) in dll.iter().enumerate() {
        assert!(*node == i as u64);
    }

    let i = rng.gen_range(0, MAX_SIZE - 1);
    let j = rng.gen_range(0, MAX_SIZE - 1);
    dll.remove(i).unwrap();
    dll.remove(j).unwrap();
    let v = rng.gen::<u64>();
    dll.push_back(v);
    assert!(dll.remove(j).unwrap() == v);
    let v = rng.gen::<u64>();
    dll.push_back(v);
    assert!(dll.tail == j as u32);

    let v = rng.gen::<u64>();
    dll.push_back(v);
    assert!(dll.tail == i as u32);

    let mut evens = vec![];
    for (p, node) in dll.iter() {
        if node % 2 == 0 {
            evens.push(p);
        }
    }
    for p in evens.into_iter() {
        dll.remove(p);
    }

    for (_p, node) in dll.iter() {
        assert!(node % 2 == 1);
    }
}
