#![feature(test)]

extern crate test;

#[cfg(test)]
mod bench_tests {
    use rand::seq::SliceRandom;
    use rand::{self, Rng};
    use sokoban::node_allocator::FromSlice;
    use sokoban::node_allocator::NodeAllocatorMap;
    use sokoban::*;
    use std::collections::BTreeMap;
    use std::collections::HashMap;
    use test::Bencher;
    use sokoban::binary_heap::Heap;
    use std::collections::BinaryHeap;

    const MAX_SIZE: usize = 20001;
    const NUM_BUCKETS: usize = MAX_SIZE >> 2;
    const NUM_NODES: usize = (MAX_SIZE << 1) + 1;

    type RBTree = RedBlackTree<u128, u128, MAX_SIZE>;
    type SHashMap = HashTable<u128, u128, NUM_BUCKETS, MAX_SIZE>;
    type AVLTreeMap = AVLTree<u128, u128, MAX_SIZE>;
    type CritbitTree = Critbit<u128, NUM_NODES, MAX_SIZE>;

    const NUM_BUCKETS_1K: usize = 1000;
    const NUM_NODES_1K: usize = (1001 << 1) + 1;

    type RBTree1K = RedBlackTree<u128, u128, 1001>;
    type SHashMap1K = HashTable<u128, u128, NUM_BUCKETS_1K, 2001>;
    type AVLTreeMap1K = AVLTree<u128, u128, 1001>;
    type CritbitTree1K = Critbit<u128, NUM_NODES_1K, 1001>;

    #[bench]
    fn bench_std_btree_map_insert_1000_u128(b: &mut Bencher) {
        let mut rng = rand::thread_rng();
        let mut m = BTreeMap::new();
        b.iter(|| {
            for v in 0..1000 {
                m.insert(v as u128, rng.gen::<u128>());
            }
        })
    }

    #[bench]
    fn bench_std_hash_map_insert_1000_u128(b: &mut Bencher) {
        let mut rng = rand::thread_rng();
        let mut m = HashMap::new();
        b.iter(|| {
            for v in 0..1000 {
                m.insert(v as u128, rng.gen::<u128>());
            }
        })
    }

    #[bench]
    fn bench_std_binary_heap_insert_1000_u128(b: &mut Bencher) {
        let mut rng = rand::thread_rng();
        let mut heap = BinaryHeap::<u128>::default();
        b.iter(|| {
            for v in 0..1000 {
                heap.push(rng.gen::<u128>());
            }
        })
    }

    #[bench]
    fn bench_sokoban_red_black_tree_insert_1000_u128(b: &mut Bencher) {
        let mut rng = rand::thread_rng();
        let mut buf = vec![0u8; std::mem::size_of::<RBTree1K>()];
        let m = RBTree1K::new_from_slice(buf.as_mut_slice());
        b.iter(|| {
            for v in 0..1000 {
                m.insert(v as u128, rng.gen::<u128>());
            }
        })
    }

    #[bench]
    fn bench_sokoban_hash_map_insert_1000_u128(b: &mut Bencher) {
        let mut rng = rand::thread_rng();
        let mut buf = vec![0u8; std::mem::size_of::<SHashMap1K>()];
        let m = SHashMap1K::new_from_slice(buf.as_mut_slice());
        b.iter(|| {
            for v in 0..1000 {
                m.insert(v as u128, rng.gen::<u128>());
            }
        })
    }

    #[bench]
    fn bench_sokoban_critbit_insert_1000_u128(b: &mut Bencher) {
        let mut rng = rand::thread_rng();
        let mut buf = vec![0u8; std::mem::size_of::<CritbitTree1K>()];
        let m = CritbitTree1K::new_from_slice(buf.as_mut_slice());
        b.iter(|| {
            for v in 0..1000 {
                m.insert(v as u128, rng.gen::<u128>());
            }
        })
    }

    #[bench]
    fn bench_sokoban_avl_tree_insert_1000_u128(b: &mut Bencher) {
        let mut rng = rand::thread_rng();
        let mut buf = vec![0u8; std::mem::size_of::<AVLTreeMap1K>()];
        let m = AVLTreeMap1K::new_from_slice(buf.as_mut_slice());
        b.iter(|| {
            for v in 0..1000 {
                m.insert(v as u128, rng.gen::<u128>());
            }
        })
    }

    #[bench]
    fn bench_sokoban_binary_heap_insert_1000_u128(b: &mut Bencher) {
        let mut rng = rand::thread_rng();
        let mut sokoban_heap = Heap::<u128, u128, 1001>::default();
        let mut slice: Vec<u128> = (0..1000).collect();

        b.iter(|| {
            for v in 0..1000 {
                sokoban_heap.push(rng.gen::<u128>());
            }
        })
    }

    #[bench]
    fn bench_sokoban_red_black_tree_insert_1000_u128_stack(b: &mut Bencher) {
        let mut rng = rand::thread_rng();
        let mut m = RBTree1K::new();
        b.iter(|| {
            for v in 0..1000 {
                m.insert(v as u128, rng.gen::<u128>());
            }
        })
    }

    #[bench]
    fn bench_sokoban_hash_map_insert_1000_u128_stack(b: &mut Bencher) {
        let mut rng = rand::thread_rng();
        let mut m = SHashMap1K::new();
        b.iter(|| {
            for v in 0..1000 {
                m.insert(v as u128, rng.gen::<u128>());
            }
        })
    }

    #[bench]
    fn bench_sokoban_critbit_insert_1000_u128_stack(b: &mut Bencher) {
        let mut rng = rand::thread_rng();
        let mut m = CritbitTree1K::new();
        b.iter(|| {
            for v in 0..1000 {
                m.insert(v as u128, rng.gen::<u128>());
            }
        })
    }

    #[bench]
    fn bench_sokoban_avl_tree_insert_1000_u128_stack(b: &mut Bencher) {
        let mut rng = rand::thread_rng();
        let mut m = AVLTreeMap1K::new();
        b.iter(|| {
            for v in 0..1000 {
                m.insert(v as u128, rng.gen::<u128>());
            }
        })
    }

    #[bench]
    fn bench_std_btree_map_insert_20000_u128(b: &mut Bencher) {
        let mut rng = rand::thread_rng();
        let mut m = BTreeMap::new();
        b.iter(|| {
            for v in 0..20000 {
                m.insert(v as u128, rng.gen::<u128>());
            }
        })
    }

    #[bench]
    fn bench_std_hash_map_insert_20000_u128(b: &mut Bencher) {
        let mut rng = rand::thread_rng();
        let mut m = HashMap::new();
        b.iter(|| {
            for v in 0..20000 {
                m.insert(v as u128, rng.gen::<u128>());
            }
        })
    }

    #[bench]
    fn bench_std_binary_heap_insert_20000_u128(b: &mut Bencher) {
        let mut rng = rand::thread_rng();
        let mut heap = BinaryHeap::<u128>::default();
        b.iter(|| {
            for v in 0..20000 {
                heap.push(rng.gen::<u128>());
            }
        })
    }

    #[bench]
    fn bench_sokoban_red_black_tree_insert_20000_u128(b: &mut Bencher) {
        let mut rng = rand::thread_rng();
        let mut buf = vec![0u8; std::mem::size_of::<RBTree>()];
        let m = RBTree::new_from_slice(buf.as_mut_slice());
        b.iter(|| {
            for v in 0..20000 {
                m.insert(v as u128, rng.gen::<u128>());
            }
        })
    }

    #[bench]
    fn bench_sokoban_hash_map_insert_20000_u128(b: &mut Bencher) {
        let mut rng = rand::thread_rng();
        let mut buf = vec![0u8; std::mem::size_of::<SHashMap>()];
        let m = SHashMap::new_from_slice(buf.as_mut_slice());
        b.iter(|| {
            for v in 0..20000 {
                m.insert(v as u128, rng.gen::<u128>());
            }
        })
    }

    #[bench]
    fn bench_sokoban_critbit_insert_20000_u128(b: &mut Bencher) {
        let mut rng = rand::thread_rng();
        let mut buf = vec![0u8; std::mem::size_of::<CritbitTree>()];
        let m = CritbitTree::new_from_slice(buf.as_mut_slice());
        b.iter(|| {
            for v in 0..20000 {
                m.insert(v as u128, rng.gen::<u128>());
            }
        })
    }

    #[bench]
    fn bench_sokoban_avl_tree_insert_20000_u128(b: &mut Bencher) {
        let mut rng = rand::thread_rng();
        let mut buf = vec![0u8; std::mem::size_of::<AVLTreeMap>()];
        let m = AVLTreeMap::new_from_slice(buf.as_mut_slice());
        b.iter(|| {
            for v in 0..20000 {
                m.insert(v as u128, rng.gen::<u128>());
            }
        })
    }

    #[bench]
    fn bench_sokoban_binary_heap_insert_20000_u128(b: &mut Bencher) {
        let mut rng = rand::thread_rng();
        let mut sokoban_heap = Heap::<u128, u128, 1001>::default();

        b.iter(|| {
            for v in 0..1000 {
                sokoban_heap.push(rng.gen::<u128>());
            }
        })
    }

    #[bench]
    fn bench_std_btree_map_remove_1000_u128(b: &mut Bencher) {
        let mut rng = rand::thread_rng();
        let mut m = BTreeMap::new();
        let mut slice: Vec<u128> = (0..1000).collect();
        slice.shuffle(&mut rng);
        b.iter(|| {
            for v in 0..1000 {
                m.insert(v as u128, rng.gen::<u128>());
            }
            for k in slice.iter() {
                m.remove(k);
            }
        })
    }

    #[bench]
    fn bench_std_hash_map_remove_1000_u128(b: &mut Bencher) {
        let mut rng = rand::thread_rng();
        let mut m = HashMap::new();
        let mut slice: Vec<u128> = (0..1000).collect();
        slice.shuffle(&mut rng);
        b.iter(|| {
            for v in 0..1000 {
                m.insert(v as u128, rng.gen::<u128>());
            }
            for k in slice.iter() {
                m.remove(k);
            }
        })
    }

    #[bench]
    fn bench_std_binary_heap_remove_1000_u128(b: &mut Bencher) {
        let mut rng = rand::thread_rng();
        let mut heap = BinaryHeap::<u128>::default();
        let mut slice: Vec<u128> = (0..1000).collect();
        slice.shuffle(&mut rng);
        b.iter(|| {
            for v in 0..1000 {
                heap.push(rng.gen::<u128>());
            }
            for k in slice.iter() {
                heap.pop();
            }
        })
    }

    #[bench]
    fn bench_sokoban_red_black_tree_remove_1000_u128(b: &mut Bencher) {
        let mut rng = rand::thread_rng();
        let mut buf = vec![0u8; std::mem::size_of::<RBTree>()];
        let m = RBTree::new_from_slice(buf.as_mut_slice());
        let mut slice: Vec<u128> = (0..1000).collect();
        slice.shuffle(&mut rng);
        b.iter(|| {
            for v in 0..1000 {
                m.insert(v as u128, rng.gen::<u128>());
            }
            for k in slice.iter() {
                m.remove(k);
            }
        })
    }

    #[bench]
    fn bench_sokoban_hash_map_remove_1000_u128(b: &mut Bencher) {
        let mut rng = rand::thread_rng();
        let mut buf = vec![0u8; std::mem::size_of::<SHashMap>()];
        let m = SHashMap::new_from_slice(buf.as_mut_slice());
        let mut slice: Vec<u128> = (0..1000).collect();
        slice.shuffle(&mut rng);
        b.iter(|| {
            for v in 0..1000 {
                m.insert(v as u128, rng.gen::<u128>());
            }
            for k in slice.iter() {
                m.remove(k);
            }
        })
    }

    #[bench]
    fn bench_sokoban_critbit_remove_1000_u128(b: &mut Bencher) {
        let mut rng = rand::thread_rng();
        let mut buf = vec![0u8; std::mem::size_of::<CritbitTree>()];
        let m = CritbitTree::new_from_slice(buf.as_mut_slice());
        let mut slice: Vec<u128> = (0..1000).collect();
        slice.shuffle(&mut rng);
        b.iter(|| {
            for v in 0..1000 {
                m.insert(v as u128, rng.gen::<u128>());
            }
            for k in slice.iter() {
                m.remove(k);
            }
        })
    }

    #[bench]
    fn bench_sokoban_avl_tree_remove_u128(b: &mut Bencher) {
        let mut rng = rand::thread_rng();
        let mut buf = vec![0u8; std::mem::size_of::<AVLTreeMap>()];
        let m = AVLTreeMap::new_from_slice(buf.as_mut_slice());
        let mut slice: Vec<u128> = (0..1000).collect();
        slice.shuffle(&mut rng);
        b.iter(|| {
            for v in 0..1000 {
                m.insert(v as u128, rng.gen::<u128>());
            }
            for k in slice.iter() {
                m.remove(k);
            }
        })
    }

    #[bench]
    fn bench_sokoban_binary_heap_remove_1000_u128(b: &mut Bencher) {
        let mut rng = rand::thread_rng();
        let mut sokoban_heap = Heap::<u128, u128, 1001>::default();
        let mut slice: Vec<u128> = (0..1000).collect();

        b.iter(|| {
            for v in 0..1000 {
                sokoban_heap.push(rng.gen::<u128>());
            }
            for k in slice.iter() {
                sokoban_heap.pop();
            }
        })
    }
}
