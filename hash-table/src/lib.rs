use bytemuck::{Pod, Zeroable};
use node_allocator::{NodeAllocator, ZeroCopy, SENTINEL};
use std::collections::hash_map::DefaultHasher;
use std::hash::Hasher;
use std::{
    hash::Hash,
    ops::{Index, IndexMut},
};

// Register aliases
pub const PREV: u32 = 0;
pub const NEXT: u32 = 1;

#[repr(C)]
#[derive(Default, Copy, Clone)]
pub struct HashNode<
    K: Hash + PartialEq + Copy + Clone + Default + Pod + Zeroable,
    V: Default + Copy + Clone + Pod + Zeroable,
> {
    pub key: K,
    pub value: V,
}

unsafe impl<
        K: Hash + PartialEq + Copy + Clone + Default + Pod + Zeroable,
        V: Default + Copy + Clone + Pod + Zeroable,
    > Zeroable for HashNode<K, V>
{
}
unsafe impl<
        K: Hash + PartialEq + Copy + Clone + Default + Pod + Zeroable,
        V: Default + Copy + Clone + Pod + Zeroable,
    > Pod for HashNode<K, V>
{
}

impl<
        K: Hash + PartialEq + Copy + Clone + Default + Pod + Zeroable,
        V: Default + Copy + Clone + Pod + Zeroable,
    > HashNode<K, V>
{
    pub fn new(key: K, value: V) -> Self {
        Self { key, value }
    }
}

#[derive(Copy, Clone)]
pub struct HashTable<
    K: Hash + PartialEq + Copy + Clone + Default + Pod + Zeroable,
    V: Default + Copy + Clone + Pod + Zeroable,
    const NUM_BUCKETS: usize,
    const MAX_SIZE: usize,
> {
    pub sequence_number: u64,
    pub buckets: [u32; NUM_BUCKETS],
    pub allocator: NodeAllocator<HashNode<K, V>, MAX_SIZE, 2>,
}

unsafe impl<
        K: Hash + PartialEq + Copy + Clone + Default + Pod + Zeroable,
        V: Default + Copy + Clone + Pod + Zeroable,
        const NUM_BUCKETS: usize,
        const MAX_SIZE: usize,
    > Zeroable for HashTable<K, V, NUM_BUCKETS, MAX_SIZE>
{
}
unsafe impl<
        K: Hash + PartialEq + Copy + Clone + Default + Pod + Zeroable,
        V: Default + Copy + Clone + Pod + Zeroable,
        const NUM_BUCKETS: usize,
        const MAX_SIZE: usize,
    > Pod for HashTable<K, V, NUM_BUCKETS, MAX_SIZE>
{
}

impl<
        K: Hash + PartialEq + Copy + Clone + Default + Pod + Zeroable,
        V: Default + Copy + Clone + Pod + Zeroable,
        const NUM_BUCKETS: usize,
        const MAX_SIZE: usize,
    > ZeroCopy for HashTable<K, V, NUM_BUCKETS, MAX_SIZE>
{
}

impl<
        K: Hash + PartialEq + Copy + Clone + Default + Pod + Zeroable,
        V: Default + Copy + Clone + Pod + Zeroable,
        const NUM_BUCKETS: usize,
        const MAX_SIZE: usize,
    > Default for HashTable<K, V, NUM_BUCKETS, MAX_SIZE>
{
    fn default() -> Self {
        HashTable {
            sequence_number: 0,
            buckets: [SENTINEL; NUM_BUCKETS],
            allocator: NodeAllocator::<HashNode<K, V>, MAX_SIZE, 2>::default(),
        }
    }
}

impl<
        K: Hash + PartialEq + Copy + Clone + Default + Pod + Zeroable,
        V: Default + Copy + Clone + Pod + Zeroable,
        const NUM_BUCKETS: usize,
        const MAX_SIZE: usize,
    > HashTable<K, V, NUM_BUCKETS, MAX_SIZE>
{
    pub fn new() -> Self {
        Self::default()
    }

    pub fn new_from_slice(slice: &mut [u8]) -> &mut Self {
        let tab = Self::load_mut_bytes(slice).unwrap();
        tab.allocator.init_default();
        tab
    }

    pub fn size(&self) -> usize {
        self.allocator.size as usize
    }

    pub fn get_next(&self, index: u32) -> u32 {
        self.allocator.get_register(index, NEXT)
    }

    pub fn get_prev(&self, index: u32) -> u32 {
        self.allocator.get_register(index, PREV)
    }

    pub fn get_node(&self, index: u32) -> &HashNode<K, V> {
        self.allocator.get(index).get_value()
    }

    pub fn get_node_mut(&mut self, index: u32) -> &mut HashNode<K, V> {
        self.allocator.get_mut(index).get_value_mut()
    }

    pub fn insert(&mut self, key: K, value: V) -> Option<u32> {
        let mut hasher = DefaultHasher::new();
        key.hash(&mut hasher);
        let bucket_index = hasher.finish() as usize % NUM_BUCKETS;
        let head = self.buckets[bucket_index];
        let mut curr_node = head;
        while curr_node != SENTINEL {
            let node = self.get_node(curr_node);
            if node.key == key {
                self.get_node_mut(curr_node).value = value;
                return Some(curr_node);
            } else {
                curr_node = self.get_next(curr_node);
            }
        }
        if self.size() >= MAX_SIZE - 1 {
            return None;
        }
        let node_index = self.allocator.add_node(HashNode::new(key, value));
        self.buckets[bucket_index] = node_index;
        if head != SENTINEL {
            self.allocator.connect(node_index, head, NEXT, PREV);
        }
        Some(node_index)
    }

    pub fn remove(&mut self, key: &K) -> Option<V> {
        let mut hasher = DefaultHasher::new();
        key.hash(&mut hasher);
        let bucket_index = hasher.finish() as usize % NUM_BUCKETS;
        let head = self.buckets[bucket_index];
        let mut curr_node = self.buckets[bucket_index];
        while curr_node != SENTINEL {
            let node = self.get_node(curr_node);
            if node.key == *key {
                let val = node.value;
                let prev = self.get_prev(curr_node);
                let next = self.get_next(curr_node);
                self.allocator.clear_register(curr_node, PREV);
                self.allocator.clear_register(curr_node, NEXT);
                self.allocator.remove_node(curr_node);
                if head == curr_node {
                    assert!(prev == SENTINEL);
                    self.buckets[bucket_index] = next;
                }
                self.allocator.connect(prev, next, NEXT, PREV);
                return Some(val);
            } else {
                curr_node = self.get_next(curr_node);
            }
        }
        None
    }

    pub fn contains(&self, key: &K) -> bool {
        let mut hasher = DefaultHasher::new();
        key.hash(&mut hasher);
        let bucket_index = hasher.finish() as usize % NUM_BUCKETS;
        let mut curr_node = self.buckets[bucket_index];
        while curr_node != SENTINEL {
            let node = self.get_node(curr_node);
            if node.key == *key {
                return true;
            } else {
                curr_node = self.get_next(curr_node);
            }
        }
        false
    }

    pub fn get(&self, key: &K) -> Option<&V> {
        let mut hasher = DefaultHasher::new();
        key.hash(&mut hasher);
        let bucket_index = hasher.finish() as usize % NUM_BUCKETS;
        let mut curr_node = self.buckets[bucket_index];
        while curr_node != SENTINEL {
            let node = self.get_node(curr_node);
            if node.key == *key {
                return Some(&node.value);
            } else {
                curr_node = self.get_next(curr_node);
            }
        }
        None
    }

    pub fn get_mut(&mut self, key: &K) -> Option<&mut V> {
        let mut hasher = DefaultHasher::new();
        key.hash(&mut hasher);
        let bucket_index = hasher.finish() as usize % NUM_BUCKETS;
        let head = self.buckets[bucket_index];
        let mut curr_node = head;
        while curr_node != SENTINEL {
            let node = self.get_node(curr_node);
            if node.key == *key {
                let prev = self.get_prev(curr_node);
                let next = self.get_next(curr_node);
                if curr_node != head {
                    self.allocator.clear_register(curr_node, PREV);
                    self.allocator.connect(prev, next, NEXT, PREV);
                    self.allocator.connect(curr_node, head, NEXT, PREV);
                }
                self.buckets[bucket_index] = curr_node;
                return Some(&mut self.get_node_mut(curr_node).value);
            } else {
                curr_node = self.get_next(curr_node);
            }
        }
        None
    }

    pub fn iter(&self) -> HashTableIterator<'_, K, V, NUM_BUCKETS, MAX_SIZE> {
        HashTableIterator::<K, V, NUM_BUCKETS, MAX_SIZE> {
            ht: self,
            bucket: 0,
            node: self.buckets[0],
        }
    }

}

pub struct HashTableIterator<
    'a,
    K: Hash + PartialEq + Copy + Clone + Default + Pod + Zeroable,
    V: Default + Copy + Clone + Pod + Zeroable,
    const NUM_BUCKETS: usize,
    const MAX_SIZE: usize,
> {
    pub ht: &'a HashTable<K, V, NUM_BUCKETS, MAX_SIZE>,
    pub bucket: usize,
    pub node: u32,
}

impl<
    'a,
    K: Hash + PartialEq + Copy + Clone + Default + Pod + Zeroable,
    V: Default + Copy + Clone + Pod + Zeroable,
    const NUM_BUCKETS: usize,
    const MAX_SIZE: usize,
> Iterator
    for HashTableIterator<'a, K, V, NUM_BUCKETS, MAX_SIZE>
{
    type Item = (&'a K, &'a V);

    fn next(&mut self) -> Option<Self::Item> {
        if self.bucket < NUM_BUCKETS {
            if self.node == SENTINEL {
                self.bucket += 1;
                let head = self.ht.buckets[self.bucket];
                self.node = head;
            } 
            let node = self.ht.get_node(self.node);
            self.node = self.ht.get_next(self.node);
            Some((&node.key, &node.value))
        } else {
            None
        }
    }
}

impl<
        K: Hash + PartialEq + Copy + Clone + Default + Pod + Zeroable,
        V: Default + Copy + Clone + Pod + Zeroable,
        const NUM_BUCKETS: usize,
        const MAX_SIZE: usize,
    > Index<&K> for HashTable<K, V, NUM_BUCKETS, MAX_SIZE>
{
    type Output = V;

    fn index(&self, index: &K) -> &Self::Output {
        &self.get(index).unwrap()
    }
}

impl<
        K: Hash + PartialEq + Copy + Clone + Default + Pod + Zeroable,
        V: Default + Copy + Clone + Pod + Zeroable,
        const NUM_BUCKETS: usize,
        const MAX_SIZE: usize,
    > IndexMut<&K> for HashTable<K, V, NUM_BUCKETS, MAX_SIZE>
{
    fn index_mut(&mut self, index: &K) -> &mut Self::Output {
        self.get_mut(index).unwrap()
    }
}
