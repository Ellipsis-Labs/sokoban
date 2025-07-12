#![feature(test)]

extern crate test;

#[cfg(test)]
mod bench_tests {
    use rand::seq::SliceRandom;
    use rand::{self, Rng};
    use sokoban::node_allocator::size_of_nodes;
    use sokoban::node_allocator::FromSlice;
    use sokoban::node_allocator::{NodeAllocatorMap, Superblock};
    use sokoban::red_black_tree::RBNode;
    use sokoban::*;
    use std::collections::BTreeMap;
    use std::collections::HashMap;
    use test::Bencher;

    const MAX_SIZE: usize = 20001;
    const NUM_BUCKETS: usize = MAX_SIZE >> 2;
    const NUM_NODES: usize = (MAX_SIZE << 1) + 1;

    type RBTree<'a> = StaticRedBlackTree<'a, u128, u128, MAX_SIZE>;
    type SHashMap<'a> = StaticHashTable<'a, u128, u128, NUM_BUCKETS, MAX_SIZE>;
    type AVLTreeMap<'a> = StaticAVLTree<'a, u128, u128, MAX_SIZE>;
    type CritbitTree = Critbit<u128, NUM_NODES, MAX_SIZE>;

    const NUM_BUCKETS_1K: usize = 1000;
    const NUM_NODES_1K: usize = (1001 << 1) + 1;

    type RBTree1K<'a> = StaticRedBlackTree<'a, u128, u128, 1001>;
    type SHashMap1K<'a> = StaticHashTable<'a, u128, u128, NUM_BUCKETS_1K, 2001>;
    type AVLTreeMap1K<'a> = StaticAVLTree<'a, u128, u128, 1001>;
    type CritbitTree1K = Critbit<u128, NUM_NODES_1K, 1001>;

    type RBTreeMultiArena<'a> = DynamicRedBlackTree<'a, u128, u128>;

    fn prepare_memories_for_multi_arena(
        num_arenas: usize,
        per_arena_size: usize,
    ) -> (Vec<u8>, Vec<Vec<u8>>) {
        let mut superblock_buf = vec![0u8; RBTreeMultiArena::size_of_header()];
        let arena_bufs =
            vec![vec![0u8; size_of_nodes::<RBNode<u128, u128>, 4>(per_arena_size)]; num_arenas];
        {
            let superblock = Superblock::load_mut_bytes(superblock_buf.as_mut_slice()).unwrap();
            superblock.initialize(num_arenas, num_arenas * per_arena_size, per_arena_size);
        }
        (superblock_buf, arena_bufs)
    }

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
        let mut buf = vec![0u8; RBTree1K::size_of_buffer()];
        let mut m = RBTree1K::load_from_buffer(buf.as_mut_slice());
        m.initialize();
        b.iter(|| {
            for v in 0..1000 {
                m.insert(v as u128, rng.gen::<u128>());
            }
        })
    }

    #[bench]
    fn bench_sokoban_red_black_tree_multi_arena_insert_1000_u128(b: &mut Bencher) {
        let mut rng = rand::thread_rng();
        let (mut superblock_buf, mut arena_bufs) =
            prepare_memories_for_multi_arena(4, (MAX_SIZE / 4).next_power_of_two());
        let mut arena_slices: Vec<&mut [u8]> =
            arena_bufs.iter_mut().map(|a| a.as_mut_slice()).collect();
        let mut m = RBTreeMultiArena::load_from_buffers(
            superblock_buf.as_mut_slice(),
            arena_slices.as_mut_slice(),
        );
        m.initialize();
        b.iter(|| {
            for v in 0..1000 {
                m.insert(v as u128, rng.gen::<u128>());
            }
        })
    }

    #[bench]
    fn bench_sokoban_hash_map_insert_1000_u128(b: &mut Bencher) {
        let mut rng = rand::thread_rng();
        let mut buf = vec![0u8; SHashMap1K::size_of_buffer()];
        let mut m = SHashMap1K::load_from_buffer(buf.as_mut_slice());
        m.initialize();
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
        let mut buf = vec![0u8; AVLTreeMap1K::size_of_buffer()];
        let mut m = AVLTreeMap1K::load_from_buffer(buf.as_mut_slice());
        m.initialize();
        b.iter(|| {
            for v in 0..1000 {
                m.insert(v as u128, rng.gen::<u128>());
            }
        })
    }

    #[bench]
    fn bench_sokoban_red_black_tree_insert_1000_u128_stack(b: &mut Bencher) {
        let mut rng = rand::thread_rng();
        let mut buf = vec![0u8; RBTree1K::size_of_buffer()];
        let mut m = RBTree1K::load_from_buffer(buf.as_mut_slice());
        m.initialize();
        b.iter(|| {
            for v in 0..1000 {
                m.insert(v as u128, rng.gen::<u128>());
            }
        })
    }

    #[bench]
    fn bench_sokoban_hash_map_insert_1000_u128_stack(b: &mut Bencher) {
        let mut rng = rand::thread_rng();
        let mut buf = vec![0u8; SHashMap1K::size_of_buffer()];
        let mut m = SHashMap1K::load_from_buffer(buf.as_mut_slice());
        m.initialize();
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
        let mut buf = vec![0u8; AVLTreeMap1K::size_of_buffer()];
        let mut m = AVLTreeMap1K::load_from_buffer(buf.as_mut_slice());
        m.initialize();
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
    fn bench_sokoban_red_black_tree_insert_20000_u128(b: &mut Bencher) {
        let mut rng = rand::thread_rng();
        let mut buf = vec![0u8; RBTree::size_of_buffer()];
        let mut m = RBTree::load_from_buffer(buf.as_mut_slice());
        m.initialize();
        b.iter(|| {
            for v in 0..20000 {
                m.insert(v as u128, rng.gen::<u128>());
            }
        })
    }

    #[bench]
    fn bench_sokoban_red_black_tree_multi_arena_insert_20000_u128(b: &mut Bencher) {
        let mut rng = rand::thread_rng();
        let (mut superblock_buf, mut arena_bufs) =
            prepare_memories_for_multi_arena(4, (MAX_SIZE / 4).next_power_of_two());
        let mut arena_slices: Vec<&mut [u8]> =
            arena_bufs.iter_mut().map(|a| a.as_mut_slice()).collect();
        let mut m = RBTreeMultiArena::load_from_buffers(
            superblock_buf.as_mut_slice(),
            arena_slices.as_mut_slice(),
        );
        m.initialize();
        b.iter(|| {
            for v in 0..20000 {
                m.insert(v as u128, rng.gen::<u128>());
            }
        })
    }

    #[bench]
    fn bench_sokoban_hash_map_insert_20000_u128(b: &mut Bencher) {
        let mut rng = rand::thread_rng();
        let mut buf = vec![0u8; SHashMap::size_of_buffer()];
        let mut m = SHashMap::load_from_buffer(buf.as_mut_slice());
        m.initialize();
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
        let mut buf = vec![0u8; AVLTreeMap::size_of_buffer()];
        let mut m = AVLTreeMap::load_from_buffer(buf.as_mut_slice());
        m.initialize();
        b.iter(|| {
            for v in 0..20000 {
                m.insert(v as u128, rng.gen::<u128>());
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
        let mut buf = vec![0u8; RBTree::size_of_buffer()];
        let mut m = RBTree::load_from_buffer(buf.as_mut_slice());
        m.initialize();
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
    fn bench_sokoban_red_black_tree_multi_arena_remove_1000_u128(b: &mut Bencher) {
        let mut rng = rand::thread_rng();
        let (mut superblock_buf, mut arena_bufs) =
            prepare_memories_for_multi_arena(4, (MAX_SIZE / 4).next_power_of_two());
        let mut arena_slices: Vec<&mut [u8]> =
            arena_bufs.iter_mut().map(|a| a.as_mut_slice()).collect();
        let mut m = RBTreeMultiArena::load_from_buffers(
            superblock_buf.as_mut_slice(),
            arena_slices.as_mut_slice(),
        );
        m.initialize();
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
        let mut buf = vec![0u8; SHashMap::size_of_buffer()];
        let mut m = SHashMap::load_from_buffer(buf.as_mut_slice());
        m.initialize();
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
        let mut buf = vec![0u8; AVLTreeMap::size_of_buffer()];
        let mut m = AVLTreeMap::load_from_buffer(buf.as_mut_slice());
        m.initialize();
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
        let mut buf = vec![0u8; RBTree::size_of_buffer()];
        let mut m = RBTree::load_from_buffer(buf.as_mut_slice());
        m.initialize();
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
    fn bench_sokoban_red_black_tree_multi_arena_lookup_20000_u128(b: &mut Bencher) {
        let mut rng = rand::thread_rng();
        let (mut superblock_buf, mut arena_bufs) =
            prepare_memories_for_multi_arena(4, (MAX_SIZE / 4).next_power_of_two());
        let mut arena_slices: Vec<&mut [u8]> =
            arena_bufs.iter_mut().map(|a| a.as_mut_slice()).collect();
        let mut m = RBTreeMultiArena::load_from_buffers(
            superblock_buf.as_mut_slice(),
            arena_slices.as_mut_slice(),
        );
        m.initialize();
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
        let mut buf = vec![0u8; SHashMap::size_of_buffer()];
        let mut m = SHashMap::load_from_buffer(buf.as_mut_slice());
        m.initialize();
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
        let mut buf = vec![0u8; AVLTreeMap::size_of_buffer()];
        let mut m = AVLTreeMap::load_from_buffer(buf.as_mut_slice());
        m.initialize();
        for v in 0..20000 {
            m.insert(v as u128, rng.gen::<u128>());
        }
        b.iter(|| {
            for v in 0..20000 {
                m.get(&v);
            }
        })
    }
}
