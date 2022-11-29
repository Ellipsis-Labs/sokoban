use crate::node_allocator::{
    FromSlice, NodeAllocator, NodeAllocatorMap, NodeField, ZeroCopy, SENTINEL,
};
use bytemuck::{Pod, Zeroable};
use std::collections::hash_map::DefaultHasher;
use std::hash::Hasher;
use std::{
    hash::Hash,
    ops::{Index, IndexMut},
};

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

#[repr(C)]
#[derive(Copy, Clone)]
pub struct HashTable<
    K: Hash + PartialEq + Copy + Clone + Default + Pod + Zeroable,
    V: Default + Copy + Clone + Pod + Zeroable,
    const NUM_BUCKETS: usize,
    const MAX_SIZE: usize,
> {
    pub buckets: [u32; NUM_BUCKETS],
    pub allocator: NodeAllocator<HashNode<K, V>, MAX_SIZE, 4>,
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
        Self::assert_proper_alignment();
        HashTable {
            buckets: [SENTINEL; NUM_BUCKETS],
            allocator: NodeAllocator::<HashNode<K, V>, MAX_SIZE, 4>::default(),
        }
    }
}

impl<
        K: Hash + PartialEq + Copy + Clone + Default + Pod + Zeroable,
        V: Default + Copy + Clone + Pod + Zeroable,
        const NUM_BUCKETS: usize,
        const MAX_SIZE: usize,
    > NodeAllocatorMap<K, V> for HashTable<K, V, NUM_BUCKETS, MAX_SIZE>
{
    fn insert(&mut self, key: K, value: V) -> Option<u32> {
        self._insert(key, value)
    }

    fn remove(&mut self, key: &K) -> Option<V> {
        self._remove(key)
    }

    fn contains(&self, key: &K) -> bool {
        self.get(key).is_some()
    }

    fn get(&self, key: &K) -> Option<&V> {
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

    fn get_mut(&mut self, key: &K) -> Option<&mut V> {
        let mut hasher = DefaultHasher::new();
        key.hash(&mut hasher);
        let bucket_index = hasher.finish() as usize % NUM_BUCKETS;
        let head = self.buckets[bucket_index];
        let mut curr_node = head;
        while curr_node != SENTINEL {
            let node = self.get_node(curr_node);
            if node.key == *key {
                // If get_mut is called, we move the matched node to the front of the queue
                let prev = self.get_prev(curr_node);
                let next = self.get_next(curr_node);
                if curr_node != head {
                    self.allocator
                        .clear_register(curr_node, NodeField::Left as u32);
                    self.allocator.connect(
                        prev,
                        next,
                        NodeField::Right as u32,
                        NodeField::Left as u32,
                    );
                    self.allocator.connect(
                        curr_node,
                        head,
                        NodeField::Right as u32,
                        NodeField::Left as u32,
                    );
                }
                self.buckets[bucket_index] = curr_node;
                return Some(&mut self.get_node_mut(curr_node).value);
            } else {
                curr_node = self.get_next(curr_node);
            }
        }
        None
    }

    fn size(&self) -> usize {
        self.allocator.size as usize
    }

    fn len(&self) -> usize {
        self.allocator.size as usize
    }

    fn capacity(&self) -> usize {
        MAX_SIZE
    }

    fn iter(&self) -> Box<dyn DoubleEndedIterator<Item = (&K, &V)> + '_> {
        Box::new(self._iter())
    }

    fn iter_mut(&mut self) -> Box<dyn DoubleEndedIterator<Item = (&K, &mut V)> + '_> {
        Box::new(self._iter_mut())
    }
}

impl<
        K: Hash + PartialEq + Copy + Clone + Default + Pod + Zeroable,
        V: Default + Copy + Clone + Pod + Zeroable,
        const NUM_BUCKETS: usize,
        const MAX_SIZE: usize,
    > FromSlice for HashTable<K, V, NUM_BUCKETS, MAX_SIZE>
{
    fn new_from_slice(slice: &mut [u8]) -> &mut Self {
        Self::assert_proper_alignment();
        let tab = Self::load_mut_bytes(slice).unwrap();
        tab.initialize();
        tab
    }
}

impl<
        K: Hash + PartialEq + Copy + Clone + Default + Pod + Zeroable,
        V: Default + Copy + Clone + Pod + Zeroable,
        const NUM_BUCKETS: usize,
        const MAX_SIZE: usize,
    > HashTable<K, V, NUM_BUCKETS, MAX_SIZE>
{
    fn assert_proper_alignment() {
        assert!(NUM_BUCKETS % 2 == 0);
    }

    pub fn initialize(&mut self) {
        self.allocator.initialize();
    }

    pub fn new() -> Self {
        Self::default()
    }

    pub fn get_next(&self, index: u32) -> u32 {
        self.allocator.get_register(index, NodeField::Right as u32)
    }

    pub fn get_prev(&self, index: u32) -> u32 {
        self.allocator.get_register(index, NodeField::Left as u32)
    }

    pub fn get_node(&self, index: u32) -> &HashNode<K, V> {
        self.allocator.get(index).get_value()
    }

    pub fn get_node_mut(&mut self, index: u32) -> &mut HashNode<K, V> {
        self.allocator.get_mut(index).get_value_mut()
    }

    fn _insert(&mut self, key: K, value: V) -> Option<u32> {
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
        if self.len() >= self.capacity() {
            return None;
        }
        let node_index = self.allocator.add_node(HashNode::new(key, value));
        self.buckets[bucket_index] = node_index;
        if head != SENTINEL {
            self.allocator.connect(
                node_index,
                head,
                NodeField::Right as u32,
                NodeField::Left as u32,
            );
        }
        Some(node_index)
    }

    pub fn _remove(&mut self, key: &K) -> Option<V> {
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
                self.allocator
                    .clear_register(curr_node, NodeField::Left as u32);
                self.allocator
                    .clear_register(curr_node, NodeField::Right as u32);
                self.allocator.remove_node(curr_node);
                if head == curr_node {
                    assert!(prev == SENTINEL);
                    self.buckets[bucket_index] = next;
                }
                self.allocator
                    .connect(prev, next, NodeField::Right as u32, NodeField::Left as u32);
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

    pub fn get_addr(&self, key: &K) -> u32 {
        let mut hasher = DefaultHasher::new();
        key.hash(&mut hasher);
        let bucket_index = hasher.finish() as usize % NUM_BUCKETS;
        let mut curr_node = self.buckets[bucket_index];
        while curr_node != SENTINEL {
            let node = self.get_node(curr_node);
            if node.key == *key {
                return curr_node;
            } else {
                curr_node = self.get_next(curr_node);
            }
        }
        SENTINEL
    }

    fn _iter(&self) -> HashTableIterator<'_, K, V, NUM_BUCKETS, MAX_SIZE> {
        HashTableIterator::<K, V, NUM_BUCKETS, MAX_SIZE> {
            ht: self,
            bucket: 0,
            node: self.buckets[0],
        }
    }

    fn _iter_mut(&mut self) -> HashTableIteratorMut<'_, K, V, NUM_BUCKETS, MAX_SIZE> {
        let node = self.buckets[0];
        HashTableIteratorMut::<K, V, NUM_BUCKETS, MAX_SIZE> {
            ht: self,
            bucket: 0,
            node,
        }
    }
}

impl<
        'a,
        K: Hash + PartialEq + Copy + Clone + Default + Pod + Zeroable,
        V: Default + Copy + Clone + Pod + Zeroable,
        const NUM_BUCKETS: usize,
        const MAX_SIZE: usize,
    > IntoIterator for &'a HashTable<K, V, NUM_BUCKETS, MAX_SIZE>
{
    type Item = (&'a K, &'a V);
    type IntoIter = HashTableIterator<'a, K, V, NUM_BUCKETS, MAX_SIZE>;

    fn into_iter(self) -> Self::IntoIter {
        self._iter()
    }
}

impl<
        'a,
        K: Hash + PartialEq + Copy + Clone + Default + Pod + Zeroable,
        V: Default + Copy + Clone + Pod + Zeroable,
        const NUM_BUCKETS: usize,
        const MAX_SIZE: usize,
    > IntoIterator for &'a mut HashTable<K, V, NUM_BUCKETS, MAX_SIZE>
{
    type Item = (&'a K, &'a mut V);
    type IntoIter = HashTableIteratorMut<'a, K, V, NUM_BUCKETS, MAX_SIZE>;

    fn into_iter(self) -> Self::IntoIter {
        self._iter_mut()
    }
}

pub struct HashTableIterator<
    'a,
    K: Hash + PartialEq + Copy + Clone + Default + Pod + Zeroable,
    V: Default + Copy + Clone + Pod + Zeroable,
    const NUM_BUCKETS: usize,
    const MAX_SIZE: usize,
> {
    ht: &'a HashTable<K, V, NUM_BUCKETS, MAX_SIZE>,
    bucket: usize,
    node: u32,
}

impl<
        'a,
        K: Hash + PartialEq + Copy + Clone + Default + Pod + Zeroable,
        V: Default + Copy + Clone + Pod + Zeroable,
        const NUM_BUCKETS: usize,
        const MAX_SIZE: usize,
    > Iterator for HashTableIterator<'a, K, V, NUM_BUCKETS, MAX_SIZE>
{
    type Item = (&'a K, &'a V);

    fn next(&mut self) -> Option<Self::Item> {
        if self.bucket < NUM_BUCKETS {
            while self.node == SENTINEL {
                self.bucket += 1;
                if self.bucket == NUM_BUCKETS {
                    return None;
                }
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
        'a,
        K: Hash + PartialEq + Copy + Clone + Default + Pod + Zeroable,
        V: Default + Copy + Clone + Pod + Zeroable,
        const NUM_BUCKETS: usize,
        const MAX_SIZE: usize,
    > DoubleEndedIterator for HashTableIterator<'a, K, V, NUM_BUCKETS, MAX_SIZE>
{
    fn next_back(&mut self) -> Option<Self::Item> {
        None
    }
}

pub struct HashTableIteratorMut<
    'a,
    K: Hash + PartialEq + Copy + Clone + Default + Pod + Zeroable,
    V: Default + Copy + Clone + Pod + Zeroable,
    const NUM_BUCKETS: usize,
    const MAX_SIZE: usize,
> {
    ht: &'a mut HashTable<K, V, NUM_BUCKETS, MAX_SIZE>,
    bucket: usize,
    node: u32,
}

impl<
        'a,
        K: Hash + PartialEq + Copy + Clone + Default + Pod + Zeroable,
        V: Default + Copy + Clone + Pod + Zeroable,
        const NUM_BUCKETS: usize,
        const MAX_SIZE: usize,
    > Iterator for HashTableIteratorMut<'a, K, V, NUM_BUCKETS, MAX_SIZE>
{
    type Item = (&'a K, &'a mut V);

    fn next(&mut self) -> Option<Self::Item> {
        if self.bucket < NUM_BUCKETS {
            while self.node == SENTINEL {
                self.bucket += 1;
                if self.bucket == NUM_BUCKETS {
                    return None;
                }
                let head = self.ht.buckets[self.bucket];
                self.node = head;
            }
            let ptr = self.node;
            self.node = self.ht.get_next(self.node);
            // TODO: How does one remove this unsafe?
            unsafe {
                let node =
                    (*self.ht.allocator.nodes.as_mut_ptr().add((ptr - 1) as usize)).get_value_mut();
                Some((&node.key, &mut node.value))
            }
        } else {
            None
        }
    }
}

impl<
        'a,
        K: Hash + PartialEq + Copy + Clone + Default + Pod + Zeroable,
        V: Default + Copy + Clone + Pod + Zeroable,
        const NUM_BUCKETS: usize,
        const MAX_SIZE: usize,
    > DoubleEndedIterator for HashTableIteratorMut<'a, K, V, NUM_BUCKETS, MAX_SIZE>
{
    fn next_back(&mut self) -> Option<Self::Item> {
        None
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
        self.get(index).unwrap()
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
