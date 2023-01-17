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
#[derive(Debug, Default, Copy, Clone, PartialEq, PartialOrd, Eq, Ord)]
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

fn simulate<K: std::fmt::Debug + Clone + Copy + Zeroable + Pod + Ord, T>(expect_sorted: bool)
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
        assert!(tree.insert(k, v).is_some());
        s += 1;
        assert!(s == tree.len());
        map.insert(k, v);
        keys.push(k);
    }

    let k = rng.gen();
    let v = Widget::new_random(&mut rng);
    assert!(tree.insert(k, v).is_none());

    let mut rand_keys = keys.clone();
    rand_keys.shuffle(&mut rng);

    for k in rand_keys.iter() {
        assert!(tree.remove(k).is_some());
        s -= 1;
        map.remove(k);
    }

    assert!(tree.len() == 0);
    keys = vec![];

    for _ in 0..100 {
        assert!(s == tree.len());
        let sample = rng.gen::<f64>();
        if sample < 0.33 {
            let remaining_slots = tree.capacity() - tree.len();
            if remaining_slots == 0 {
                continue;
            }
            let num_samples = rng.gen_range(0, remaining_slots);
            for _ in 0..num_samples {
                assert!(tree.len() < tree.capacity());
                let k = rng.gen::<K>();
                let v = Widget::new_random(&mut rng);
                assert!(tree.insert(k, v).is_some());
                s += 1;
                map.insert(k, v);
                keys.push(k);
            }
        } else if sample < 0.66 {
            if tree.len() < 2 {
                continue;
            }
            let num_samples = rng.gen_range(0, tree.len() / 2);
            for _ in 0..num_samples {
                assert!(!keys.is_empty());
                let j = rng.gen_range(0, keys.len());
                let key = keys[j];
                keys.swap_remove(j);
                // assert!(rbt[&key] == map[&key]);
                assert!(tree.remove(&key).is_some());
                map.remove(&key);
                s -= 1;
            }
        } else {
            if tree.len() == 0 {
                continue;
            }
            let num_samples = rng.gen_range(0, tree.len());
            for _ in 0..num_samples {
                assert!(!keys.is_empty());
                let j = rng.gen_range(0, keys.len());
                let key = keys[j];
                let v = Widget::new_random(&mut rng);
                *tree.get_mut(&key).unwrap() = v;
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

        // Test double ended iterator
        {
            let mut node_allocator_iter = tree.iter();
            let mut btree_map_iter = new_map.iter();
            let breakpoint = rng.gen_range(1, new_map.len() - 1);

            for _ in 0..breakpoint {
                let a = node_allocator_iter.next();
                let b = btree_map_iter.next();
                assert!(a.is_some() && b.is_some());
                assert_eq!(a, b);
            }
            for _ in breakpoint..new_map.len() {
                let a = node_allocator_iter.next_back();
                let b = btree_map_iter.next_back();
                assert!(a.is_some() && b.is_some());
                assert_eq!(a, b);
            }

            assert!(node_allocator_iter.next().is_none());
            assert!(node_allocator_iter.next_back().is_none());
            assert!(btree_map_iter.next().is_none());
            assert!(btree_map_iter.next_back().is_none());
            // Do it again for good measure
            assert!(node_allocator_iter.next().is_none());
            assert!(node_allocator_iter.next_back().is_none());
            assert!(btree_map_iter.next().is_none());
            assert!(btree_map_iter.next_back().is_none());
        }
        // Test iterator can't be used again after consumed
        {
            let mut node_allocator_iter = tree.iter();
            for _ in 0..tree.len() {
                assert!(node_allocator_iter.next().is_some());
            }
            assert!(node_allocator_iter.next().is_none());
            assert!(node_allocator_iter.next_back().is_none());
            assert!(node_allocator_iter.next().is_none());
            assert!(node_allocator_iter.next_back().is_none());
            let mut node_allocator_iter = tree.iter();

            for _ in 0..tree.len() {
                assert!(node_allocator_iter.next_back().is_some());
            }
            assert!(node_allocator_iter.next_back().is_none());
            assert!(node_allocator_iter.next().is_none());
            assert!(node_allocator_iter.next_back().is_none());
            assert!(node_allocator_iter.next().is_none());
        }

        // Test double ended iterator mut
        {
            let len = new_map.len();
            let mut node_allocator_iter_mut = tree.iter_mut();
            let mut btree_map_iter_mut = new_map.iter_mut();
            let breakpoint = rng.gen_range(1, len - 1);

            for _ in 0..breakpoint {
                let a = node_allocator_iter_mut.next();
                let b = btree_map_iter_mut.next();
                assert!(a.is_some() && b.is_some());
                assert_eq!(a, b);
                let w = Widget::new_random(&mut rng);
                *a.unwrap().1 = w;
                *b.unwrap().1 = w;
            }
            for _ in breakpoint..len {
                let a = node_allocator_iter_mut.next_back();
                let b = btree_map_iter_mut.next_back();
                assert!(a.is_some() && b.is_some());
                assert_eq!(a, b);
                let w = Widget::new_random(&mut rng);
                *a.unwrap().1 = w;
                *b.unwrap().1 = w;
            }

            assert!(node_allocator_iter_mut.next().is_none());
            assert!(node_allocator_iter_mut.next_back().is_none());
            assert!(btree_map_iter_mut.next().is_none());
            assert!(btree_map_iter_mut.next_back().is_none());
            // Do it again for good measure
            assert!(node_allocator_iter_mut.next().is_none());
            assert!(node_allocator_iter_mut.next_back().is_none());
            assert!(btree_map_iter_mut.next().is_none());
            assert!(btree_map_iter_mut.next_back().is_none());
        }
    } else {
        for ((k1, v1), (k2, v2)) in new_map.iter().zip(tree.iter().sorted()) {
            assert!(*k1 == *k2);
            assert!(*v1 == *v2);
        }
    }

    let mut new_map = BTreeMap::new();
    for (k, v) in tree.iter_mut().rev() {
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

    println!("{} Size: {}", std::any::type_name::<T>(), tree.len(),);
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
