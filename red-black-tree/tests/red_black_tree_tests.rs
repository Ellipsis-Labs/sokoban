use bytemuck::Pod;
use bytemuck::Zeroable;
use node_allocator::*;
use rand::rngs::ThreadRng;
use rand::thread_rng;
use rand::{self, Rng};
use red_black_tree::*;
use std::collections::BTreeMap;


const MAX_SIZE: usize = 500;

#[tokio::test(threaded_scheduler)]
async fn test_initialize() {
    let rbt = RedBlackTree::<MAX_SIZE, u64, u64>::new();

    assert_eq!(
        rbt.sequence_number, 0,
        "Init failed to set sequence properly"
    );

    assert_eq!(rbt.root, SENTINEL, "Init failed to set head properly");
}

#[repr(C)]
#[derive(Default, Copy, Clone, PartialEq)]
struct Order {
    a: u128,
    b: u128,
    size: u64,
}

unsafe impl Zeroable for Order {}
unsafe impl Pod for Order {}

impl Order {
    pub fn new_random(r: &mut ThreadRng) -> Self {
        Self {
            a: r.gen::<u128>(),
            b: r.gen::<u128>(),
            size: r.gen::<u64>(),
        }
    }
}

#[tokio::test(threaded_scheduler)]
async fn test_simulate() {
    type RBTree = RedBlackTree<MAX_SIZE, u128, Order>;
    let mut rbt = RBTree::new();
    println!("Size: {}", std::mem::size_of::<RBTree>());
    let mut rng = thread_rng();
    let mut keys = vec![];
    let mut map = BTreeMap::new();
    let mut s = 0;
    for _ in 0..(MAX_SIZE - 1) {
        let k = rng.gen::<u128>();
        let v = Order::new_random(&mut rng);
        match rbt.insert(k, v) {
            None => assert!(false),
            _ => {}
        }
        s += 1;
        assert!(s == rbt.size());
        map.insert(k, v);
        keys.push(k);
    }

    let k = rng.gen::<u128>();
    let v = Order::new_random(&mut rng);
    match rbt.insert(k, v) {
        None => println!("Cannot insert when full"),
        _ => {
            assert!(false);
        }
    }



    for k in keys.iter() {
        match rbt.remove(k) {
            None => assert!(false),
            _ => {}
        }
        s -= 1;
        map.remove(k);
    }
    keys = vec![];

    for _i in 0..(MAX_SIZE >> 1) {
        let k = rng.gen::<u128>();
        let v = Order::new_random(&mut rng);
        if rbt.insert(k, v) == None {
            assert!(false);
        }
        s += 1;
        map.insert(k, v);
        keys.push(k);
    }

    for _ in 0..100000 {
        assert!(s == rbt.size());
        let sample = rng.gen::<f64>();
        if sample < 0.33 {
            if rbt.size() >= MAX_SIZE - 1 {
                continue;
            }
            let k = rng.gen::<u128>();
            let v = Order::new_random(&mut rng);
            match rbt.insert(k, v) {
                None => {
                    assert!(false);
                }
                _ => {}
            }
            s += 1;
            map.insert(k, v);
            keys.push(k);
        } else if sample < 0.66 {
            if keys.is_empty() {
                continue;
            }
            let j = rng.gen_range(0, keys.len());
            let key = keys[j];
            keys.swap_remove(j);
            assert!(rbt[&key] == map[&key]);
            rbt.remove(&key);
            map.remove(&key);
            s -= 1;
        } else {
            if keys.is_empty() {
                continue;
            }
            let j = rng.gen_range(0, keys.len());
            let key = keys[j];
            let v = Order::new_random(&mut rng);
            rbt.insert(key, v);
            map.insert(key, v);
        }
    }

    let nodes = rbt.inorder_traversal();
    for ((k1, v1), (k2, v2)) in map.iter().zip(nodes.iter()) {
        assert!(*k1 == *k2);
        assert!(*v1 == *v2);
    }
}
// assert!(false);
