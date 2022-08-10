use bytemuck::Pod;
use bytemuck::Zeroable;
use itertools::Itertools;
use rand::rngs::ThreadRng;
use rand::thread_rng;
use rand::{self, Rng};
use sokoban::node_allocator::FromSlice;
use sokoban::node_allocator::NodeAllocatorMap;
use sokoban::*;
use std::collections::BTreeMap;

const MAX_SIZE: usize = 20001;

#[repr(C)]
#[derive(Default, Copy, Clone, PartialEq, PartialOrd, Eq, Ord)]
struct Widget {
    a: u128,
    b: u128,
    size: u64,
}

unsafe impl Zeroable for Widget {}
unsafe impl Pod for Widget {}

impl Widget {
    pub fn new_random(r: &mut ThreadRng) -> Self {
        Self {
            a: r.gen::<u128>(),
            b: r.gen::<u128>(),
            size: r.gen::<u64>(),
        }
    }
}

fn simulate<'a, T>(expect_sorted: bool)
where
    T: FromSlice + NodeAllocatorMap<u128, Widget>,
{
    let mut buf = vec![0u8; std::mem::size_of::<T>()];
    let tree = T::new_from_slice(buf.as_mut_slice());
    println!(
        "{} Memory Size: {}, Capacity: {}",
        std::any::type_name::<T>(),
        std::mem::size_of::<T>(),
        MAX_SIZE - 1
    );
    let mut rng = thread_rng();
    let mut keys = vec![];
    let mut map = Box::new(BTreeMap::new());
    let mut s = 0;
    let mut v;
    for _ in 0..(MAX_SIZE - 1) {
        let k = rng.gen::<u128>();
        v = Widget::new_random(&mut rng);
        assert!(tree.insert(k, v) != None);
        s += 1;
        assert!(s == tree.size());
        map.insert(k, v);
        keys.push(k);
    }

    let k = rng.gen::<u128>();
    let v = Widget::new_random(&mut rng);
    assert!(tree.insert(k, v) == None);

    for k in keys.iter() {
        assert!(tree.remove(k) != None);
        s -= 1;
        map.remove(k);
    }
    keys = vec![];

    for _i in 0..(MAX_SIZE >> 1) {
        let k = rng.gen::<u128>();
        let v = Widget::new_random(&mut rng);
        assert!(tree.insert(k, v) != None);
        s += 1;
        map.insert(k, v);
        keys.push(k);
    }

    for _ in 0..100000 {
        assert!(s == tree.size());
        let sample = rng.gen::<f64>();
        if sample < 0.33 {
            if tree.size() >= MAX_SIZE - 1 {
                continue;
            }
            let k = rng.gen::<u128>();
            let v = Widget::new_random(&mut rng);
            assert!(tree.insert(k, v) != None);
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
            // assert!(rbt[&key] == map[&key]);
            assert!(tree.remove(&key) != None);
            map.remove(&key);
            s -= 1;
        } else {
            if keys.is_empty() {
                continue;
            }
            let j = rng.gen_range(0, keys.len());
            let key = keys[j];
            let v = Widget::new_random(&mut rng);
            assert!(tree.insert(key, v) != None);
            map.insert(key, v);
        }
    }

    if expect_sorted {
        for ((k1, v1), (k2, v2)) in map.iter().zip(tree.iter()) {
            assert!(*k1 == *k2);
            assert!(*v1 == *v2);
        }
    } else {
        for ((k1, v1), (k2, v2)) in map.iter().zip(tree.iter().sorted()) {
            assert!(*k1 == *k2);
            assert!(*v1 == *v2);
        }
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn test_simulate_red_black_tree() {
    type RBTree = RedBlackTree<u128, Widget, MAX_SIZE>;
    simulate::<RBTree>(true);
}

#[tokio::test(flavor = "multi_thread")]
async fn test_simulate_hash_table() {
    const NUM_BUCKETS: usize = MAX_SIZE >> 2;
    type HashMap = HashTable<u128, Widget, NUM_BUCKETS, MAX_SIZE>;
    simulate::<HashMap>(false);
}

#[tokio::test(flavor = "multi_thread")]
async fn test_simulate_avl_tree() {
    type AVLTreeMap = AVLTree<u128, Widget, MAX_SIZE>;
    simulate::<AVLTreeMap>(true);
}

#[tokio::test(flavor = "multi_thread")]
async fn test_simulate_critbit() {
    const NUM_NODES: usize = (MAX_SIZE << 1) + 1;
    type CritbitTree = Critbit<Widget, NUM_NODES, MAX_SIZE>;
    simulate::<CritbitTree>(true);
}
