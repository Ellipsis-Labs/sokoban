use bytemuck::Pod;
use bytemuck::Zeroable;
use itertools::Itertools;
use rand::distributions::Standard;
use rand::prelude::Distribution;
use rand::rngs::ThreadRng;
use rand::seq::SliceRandom;
use rand::thread_rng;
use rand::{self, Rng};
use sokoban::node_allocator::FromSlice;
use sokoban::node_allocator::NodeAllocatorMap;
use sokoban::*;
use std::collections::BTreeMap;

const MAX_SIZE: usize = 20000;

#[repr(C)]
#[derive(Default, Copy, Clone, PartialEq, PartialOrd, Eq, Ord)]
struct Widget {
    a: u128,
    b: u128,
    c: u64,
    d: u64,
}

unsafe impl Zeroable for Widget {}
unsafe impl Pod for Widget {}

impl Widget {
    pub fn new_random(r: &mut ThreadRng) -> Self {
        Self {
            a: r.gen::<u128>(),
            b: r.gen::<u128>(),
            c: r.gen::<u64>(),
            d: r.gen::<u64>(),
        }
    }
}

fn simulate<K: Clone + Copy + Zeroable + Pod + Ord, T>(expect_sorted: bool)
where
    T: Copy + FromSlice + NodeAllocatorMap<K, Widget>,
    Standard: Distribution<K>,
{
    let mut buf = vec![0u8; std::mem::size_of::<T>()];
    let tree = T::new_from_slice(buf.as_mut_slice());
    println!(
        "{} Memory Size: {}, Capacity: {}",
        std::any::type_name::<T>(),
        std::mem::size_of::<T>(),
        MAX_SIZE
    );
    let mut rng = thread_rng();
    let mut keys = vec![];
    let mut map = Box::new(BTreeMap::new());
    let mut s = 0;
    let mut v;
    for _ in 0..(MAX_SIZE) {
        let k = rng.gen::<K>();
        v = Widget::new_random(&mut rng);
        assert!(tree.insert(k, v) != None);
        s += 1;
        assert!(s == tree.size());
        map.insert(k, v);
        keys.push(k);
    }

    let k = rng.gen();
    let v = Widget::new_random(&mut rng);
    assert!(tree.insert(k, v) == None);

    let mut rand_keys = keys.clone();
    rand_keys.shuffle(&mut rng);

    for k in rand_keys.iter() {
        assert!(tree.remove(k) != None);
        s -= 1;
        map.remove(k);
    }

    assert!(tree.size() == 0);
    keys = vec![];

    for _ in 0..100 {
        assert!(s == tree.size());
        let sample = rng.gen::<f64>();
        if sample < 0.33 {
            let remaining_slots = MAX_SIZE - tree.size();
            if remaining_slots == 0 {
                continue;
            }
            let num_samples = rng.gen_range(0, remaining_slots);
            for _ in 0..num_samples {
                assert!(tree.size() < MAX_SIZE);
                let k = rng.gen::<K>();
                let v = Widget::new_random(&mut rng);
                assert!(tree.insert(k, v) != None);
                s += 1;
                map.insert(k, v);
                keys.push(k);
            }
        } else if sample < 0.66 {
            if tree.size() < 2 {
                continue;
            }
            let num_samples = rng.gen_range(0, tree.size() / 2);
            for _ in 0..num_samples {
                assert!(!keys.is_empty());
                let j = rng.gen_range(0, keys.len());
                let key = keys[j];
                keys.swap_remove(j);
                // assert!(rbt[&key] == map[&key]);
                assert!(tree.remove(&key) != None);
                map.remove(&key);
                s -= 1;
            }
        } else {
            if tree.size() == 0 {
                continue;
            }
            let num_samples = rng.gen_range(0, tree.size());
            for _ in 0..num_samples {
                assert!(!keys.is_empty());
                let j = rng.gen_range(0, keys.len());
                let key = keys[j];
                let v = Widget::new_random(&mut rng);
                assert!(tree.insert(key, v) != None);
                map.insert(key, v);
            }
        }
    }
    if expect_sorted {
        for ((k1, v1), (k2, v2)) in map.iter().zip(tree.iter()) {
            assert!(*k1 == *k2);
            assert!(*v1 == *v2);
        }
        for ((k1, v1), (k2, v2)) in map.iter().rev().zip(tree.iter().rev()) {
            assert!(*k1 == *k2);
            assert!(*v1 == *v2);
        }
    } else {
        for ((k1, v1), (k2, v2)) in map.iter().zip(tree.iter().sorted()) {
            assert!(*k1 == *k2);
            assert!(*v1 == *v2);
        }
    }

    let mut new_map = BTreeMap::new();
    for (k, v) in tree.iter_mut() {
        let w = Widget::new_random(&mut rng);
        *v = w;
        new_map.insert(*k, w);
    }

    if expect_sorted {
        for ((k1, v1), (k2, v2)) in new_map.iter().zip(tree.iter()) {
            assert!(*k1 == *k2);
            assert!(*v1 == *v2);
        }
        for ((k1, v1), (k2, v2)) in new_map.iter().rev().zip(tree.iter().rev()) {
            assert!(*k1 == *k2);
            assert!(*v1 == *v2);
        }
    } else {
        for ((k1, v1), (k2, v2)) in new_map.iter().zip(tree.iter().sorted()) {
            assert!(*k1 == *k2);
            assert!(*v1 == *v2);
        }
    }
    println!("{} Size: {}", std::any::type_name::<T>(), tree.size(),);
}

#[tokio::test(flavor = "multi_thread")]
async fn test_simulate_red_black_tree() {
    type RBTree = RedBlackTree<u64, Widget, MAX_SIZE>;
    simulate::<u64, RBTree>(true);
}

#[tokio::test(flavor = "multi_thread")]
async fn test_simulate_hash_table() {
    const NUM_BUCKETS: usize = MAX_SIZE >> 2;
    type HashMap = HashTable<u64, Widget, NUM_BUCKETS, MAX_SIZE>;
    simulate::<u64, HashMap>(false);
}

#[tokio::test(flavor = "multi_thread")]
async fn test_simulate_avl_tree() {
    type AVLTreeMap = AVLTree<u64, Widget, MAX_SIZE>;
    simulate::<u64, AVLTreeMap>(true);
}

#[tokio::test(flavor = "multi_thread")]
async fn test_simulate_critbit() {
    const NUM_NODES: usize = MAX_SIZE << 1;
    type CritbitTree = Critbit<Widget, NUM_NODES, MAX_SIZE>;
    simulate::<u128, CritbitTree>(true);
}
