use bytemuck::Pod;
use bytemuck::Zeroable;
use itertools::Itertools;
use rand::rngs::ThreadRng;
use rand::seq::SliceRandom;
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

fn simulate<T>(expect_sorted: bool)
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
            let remaining_slots = MAX_SIZE - 1 - tree.size();
            if remaining_slots == 0 {
                continue;
            }
            let num_samples = rng.gen_range(0, remaining_slots);
            for _ in 0..num_samples {
                assert!(tree.size() < MAX_SIZE - 1);
                let k = rng.gen::<u128>();
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
    println!("{} Size: {}", std::any::type_name::<T>(), tree.size(),);

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

#[cfg(test)]
pub mod binary_heap_test {
    use rand::prelude::*;
    use sokoban::binary_heap::*;
    use std::collections::BinaryHeap;
    use std::vec;

    #[test]
    fn max_heap_test() {
        const MAX_SIZE: usize = 10001;
        let mut heap = Heap::<u64, u64, MAX_SIZE>::default();
        let mut s = heap.size;
        let mut rng = rand::thread_rng();
        let mut vals: Vec<u64> = vec![];

        for _ in 0..(MAX_SIZE) {
            let n: u64 = rng.gen::<u64>();
            heap._push(n.into());
            vals.push(n.into());
            s += 1;
            assert!(s == heap._size());
        }

        assert_eq!(Some(&heap.nodes[0].key), vals.iter().max());

        for _ in 0..(MAX_SIZE / 2) {
            let old_max = heap.nodes[0].key;
            let index = vals.iter().position(|x| *x == old_max).unwrap();
            vals.remove(index);
            heap._pop(); // this is the problem
            let new_max = heap.nodes[0].key;
            assert_eq!(vals.iter().max(), Some(&new_max));
            s -= 1;
            assert!(s == heap._size());
        }
    }

    #[test]
    fn min_heap_test() {
        const MAX_SIZE: usize = 10001;
        let mut heap = Heap::<u64, u64, MAX_SIZE>::default();
        let mut s = heap.size;
        let mut rng = rand::thread_rng();
        let mut vals: Vec<u64> = vec![];

        for _ in 0..(MAX_SIZE) {
            let n: u64 = rng.gen::<u64>();
            heap._push_min(n.into());
            vals.push(n.into());
            s += 1;
            assert!(s == heap._size());
        }
        assert_eq!(Some(&heap.nodes[0].key), vals.iter().min());
    }

    #[test]
    fn test_against_normal_heap() {
        const MAX_SIZE: usize = 15;
        let mut std_heap = BinaryHeap::<u128>::new();
        let mut rng = rand::thread_rng();
        let mut sokoban_heap = Heap::<u128, u128, MAX_SIZE>::default();
        let mut vals: Vec<u128> = vec![];

        for _ in 0..(MAX_SIZE - 1) {
            let rand = rng.gen::<u128>();
            std_heap.push(rand);
            sokoban_heap._push(rand);
            vals.push(rand);
        }

        let mut stl_heap_to_vector = std_heap.into_vec().into_iter();

        let mut std_heap_arr: [u128; MAX_SIZE] = [1; MAX_SIZE];

        for i in 0..sokoban_heap.size {
            std_heap_arr[i as usize] = stl_heap_to_vector.next().unwrap();
        }

        let mut sokoban_heap_arr:  [u128; MAX_SIZE] = [1; MAX_SIZE];

        for i in 0..(MAX_SIZE - 1) {
            sokoban_heap_arr[i] = sokoban_heap.nodes[i].key;
        }

        println!("STL Heap");

        println!("{:?}", std_heap_arr);

        println!("SOKOBAN Heap");

        println!("{:?}", sokoban_heap_arr);

        println!("Vector, in order of pushes!");

        println!("{:?}", vals);

        for i in 0..sokoban_heap.size as usize {
            if Some(sokoban_heap.nodes[i].key) != Some(std_heap_arr[i]) {
                panic!("not equal!")
            }
        }
    }

    #[test]
    fn peek_mut_test() {
        let mut heap = Heap::<u32, u32, 10>::default();

        heap._push(3);
        heap._push(5);
        heap._push(10);

        assert_eq!(heap._size(), 3);
        assert_eq!(heap._is_empty(), false);
        assert_eq!(heap.nodes[0].key, 10);

        *heap.peek_mut().unwrap() = 9;

        println!("{:?}", heap.nodes);
    }

    #[test]
    fn iter_test() {
        let mut heap = Heap::<u32, u32, 4>::default();

        heap._push(3);
        heap._push(5);
        heap._push(10);

        assert_eq!(heap._size(), 3);
        assert_eq!(heap._is_empty(), false);
        assert_eq!(heap.nodes[0].key, 10);

        let next = heap._iter().next();
        println!("{:?}", next)
    }
}
