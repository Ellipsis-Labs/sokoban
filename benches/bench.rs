#![feature(test)]

extern crate test;

#[cfg(test)]
mod bench_tests {
    use rand::seq::SliceRandom;
    use rand::{self, Rng};
    use sokoban::hash_set::HashSet;
    use sokoban::node_allocator::FromSlice;
    use sokoban::node_allocator::NodeAllocatorMap;
    use sokoban::*;
    use std::collections::HashMap;
    use std::collections::{BTreeMap, HashSet as StdHashSet};
    use test::Bencher;

    const MAX_SIZE: usize = 20001;
    const NUM_BUCKETS: usize = MAX_SIZE >> 2;
    const NUM_NODES: usize = (MAX_SIZE << 1) + 1;

    type RBTree = RedBlackTree<u128, u128, MAX_SIZE>;
    type SHashMap = HashTable<u128, u128, NUM_BUCKETS, MAX_SIZE>;
    type AVLTreeMap = AVLTree<u128, u128, MAX_SIZE>;
    type CritbitTree = Critbit<u128, NUM_NODES, MAX_SIZE>;
    type SHashSet = HashSet<u128, MAX_SIZE>;

    const NUM_BUCKETS_1K: usize = 1000;
    const NUM_NODES_1K: usize = (1001 << 1) + 1;

    type RBTree1K = RedBlackTree<u128, u128, 1001>;
    type SHashMap1K = HashTable<u128, u128, NUM_BUCKETS_1K, 2001>;
    type AVLTreeMap1K = AVLTree<u128, u128, 1001>;
    type CritbitTree1K = Critbit<u128, NUM_NODES_1K, 1001>;
    type SHashSet1k = HashSet<u128, 1001>;

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
    fn bench_sokoban_hash_set_insert_1000_u128(b: &mut Bencher) {
        let mut rng = rand::thread_rng();
        let mut buf = vec![0u8; std::mem::size_of::<SHashSet1k>()];
        let s = SHashSet1k::new_from_slice(buf.as_mut_slice());
        b.iter(|| {
            for _ in 0..1000 {
                s.insert(rng.gen::<u128>());
            }
        })
    }

    #[bench]
    fn bench_std_hash_set_insert_1000_u128(b: &mut Bencher) {
        let mut rng = rand::thread_rng();
        let mut s = StdHashSet::<u128>::new();
        b.iter(|| {
            for _ in 0..1000 {
                s.insert(rng.gen::<u128>());
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
    fn bench_sokoban_hash_set_insert_1000_u128_stack(b: &mut Bencher) {
        let mut rng = rand::thread_rng();
        let mut s = SHashSet1k::new();
        b.iter(|| {
            for _ in 0..1000 {
                s.insert(rng.gen::<u128>());
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
    fn bench_sokoban_hash_set_insert_20000_u128(b: &mut Bencher) {
        let mut rng = rand::thread_rng();
        let mut buf = vec![0u8; std::mem::size_of::<SHashSet>()];
        let s = SHashSet::new_from_slice(buf.as_mut_slice());
        b.iter(|| {
            for _ in 0..20000 {
                s.insert(rng.gen::<u128>());
            }
        })
    }

    #[bench]
    fn bench_std_hash_set_insert_20000_u128(b: &mut Bencher) {
        let mut rng = rand::thread_rng();
        let mut s = StdHashSet::<u128>::new();
        b.iter(|| {
            for _ in 0..20000 {
                s.insert(rng.gen::<u128>());
            }
        })
    }

    #[bench]
    fn bench_std_btree_map_remove_1000_u128(b: &mut Bencher) {
        let mut rng = rand::thread_rng();
        let mut m = BTreeMap::new();
        let mut slice: Vec<u128> = (0..1000).collect();
        slice.shuffle(&mut rng);
        for v in 0..1000 {
            m.insert(v as u128, rng.gen::<u128>());
        }
        b.iter(|| {
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
        for v in 0..1000 {
            m.insert(v as u128, rng.gen::<u128>());
        }
        b.iter(|| {
            for k in slice.iter() {
                m.remove(k);
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
        for v in 0..1000 {
            m.insert(v as u128, rng.gen::<u128>());
        }
        b.iter(|| {
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
        for v in 0..1000 {
            m.insert(v as u128, rng.gen::<u128>());
        }
        b.iter(|| {
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
        for v in 0..1000 {
            m.insert(v as u128, rng.gen::<u128>());
        }
        b.iter(|| {
            for k in slice.iter() {
                m.remove(k);
            }
        })
    }

    #[bench]
    fn bench_sokoban_avl_tree_remove_1000_u128(b: &mut Bencher) {
        let mut rng = rand::thread_rng();
        let mut buf = vec![0u8; std::mem::size_of::<AVLTreeMap>()];
        let m = AVLTreeMap::new_from_slice(buf.as_mut_slice());
        let mut slice: Vec<u128> = (0..1000).collect();
        slice.shuffle(&mut rng);
        for v in 0..1000 {
            m.insert(v as u128, rng.gen::<u128>());
        }
        b.iter(|| {
            for k in slice.iter() {
                m.remove(k);
            }
        })
    }

    #[bench]
    fn bench_sokoban_hash_set_remove_1000_u128(b: &mut Bencher) {
        let mut rng = rand::thread_rng();
        let mut buf = vec![0u8; std::mem::size_of::<SHashSet>()];
        let s = SHashSet::new_from_slice(buf.as_mut_slice());
        let mut slice: Vec<u128> = (0..1000).collect();
        slice.shuffle(&mut rng);
        for _ in 0..1000 {
            s.insert(rng.gen::<u128>());
        }
        b.iter(|| {
            for k in slice.iter() {
                s.remove(k);
            }
        })
    }

    #[bench]
    fn bench_std_hash_set_remove_1000_u128(b: &mut Bencher) {
        let mut rng = rand::thread_rng();
        let mut s = StdHashSet::<u128>::new();
        let mut slice: Vec<u128> = (0..1000).collect();
        slice.shuffle(&mut rng);
        for _ in 0..1000 {
            s.insert(rng.gen::<u128>());
        }
        b.iter(|| {
            for k in slice.iter() {
                s.remove(k);
            }
        })
    }

    #[bench]
    fn bench_std_btree_map_lookup_20000_u128(b: &mut Bencher) {
        let mut rng = rand::thread_rng();
        let mut m = BTreeMap::new();
        for v in 0..20000 {
            m.insert(v as u128, rng.gen::<u128>());
        }
        b.iter(|| {
            for v in 0..20000 {
                m.get(&v);
            }
        })
    }

    #[bench]
    fn bench_std_hash_map_lookup_20000_u128(b: &mut Bencher) {
        let mut rng = rand::thread_rng();
        let mut m = HashMap::new();
        for v in 0..20000 {
            m.insert(v as u128, rng.gen::<u128>());
        }
        b.iter(|| {
            for v in 0..20000 {
                m.get(&v);
            }
        })
    }

    #[bench]
    fn bench_sokoban_red_black_tree_lookup_20000_u128(b: &mut Bencher) {
        let mut rng = rand::thread_rng();
        let mut buf = vec![0u8; std::mem::size_of::<RBTree>()];
        let m = RBTree::new_from_slice(buf.as_mut_slice());
        for v in 0..20000 {
            m.insert(v as u128, rng.gen::<u128>());
        }
        b.iter(|| {
            for v in 0..20000 {
                m.get(&v);
            }
        })
    }

    #[bench]
    fn bench_sokoban_hash_map_lookup_20000_u128(b: &mut Bencher) {
        let mut rng = rand::thread_rng();
        let mut buf = vec![0u8; std::mem::size_of::<SHashMap>()];
        let m = SHashMap::new_from_slice(buf.as_mut_slice());
        for v in 0..20000 {
            m.insert(v as u128, rng.gen::<u128>());
        }
        b.iter(|| {
            for v in 0..20000 {
                m.get(&v);
            }
        })
    }

    #[bench]
    fn bench_sokoban_critbit_lookup_20000_u128(b: &mut Bencher) {
        let mut rng = rand::thread_rng();
        let mut buf = vec![0u8; std::mem::size_of::<CritbitTree>()];
        let m = CritbitTree::new_from_slice(buf.as_mut_slice());
        for v in 0..20000 {
            m.insert(v as u128, rng.gen::<u128>());
        }
        b.iter(|| {
            for v in 0..20000 {
                m.get(&v);
            }
        })
    }

    #[bench]
    fn bench_sokoban_avl_tree_lookup_20000_u128(b: &mut Bencher) {
        let mut rng = rand::thread_rng();
        let mut buf = vec![0u8; std::mem::size_of::<AVLTreeMap>()];
        let m = AVLTreeMap::new_from_slice(buf.as_mut_slice());
        for v in 0..20000 {
            m.insert(v as u128, rng.gen::<u128>());
        }
        b.iter(|| {
            for v in 0..20000 {
                m.get(&v);
            }
        })
    }

    #[bench]
    fn bench_sokoban_hash_set_lookup_20000_u128(b: &mut Bencher) {
        let mut buf = vec![0u8; std::mem::size_of::<SHashSet>()];
        let s = SHashSet::new_from_slice(buf.as_mut_slice());
        for v in 0..20000 {
            s.insert(v);
        }
        b.iter(|| {
            for v in 0..20000 {
                s.contains(&v);
            }
        })
    }

    #[bench]
    fn bench_std_hash_set_lookup_20000_u128(b: &mut Bencher) {
        let mut s = StdHashSet::<u128>::new();
        for v in 0..20000 {
            s.insert(v);
        }
        b.iter(|| {
            for v in 0..20000 {
                s.contains(&v);
            }
        })
    }
}
