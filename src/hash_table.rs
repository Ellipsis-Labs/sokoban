use crate::container::AssertProperAlignment;
use crate::node_allocator::{
    MultiArenaNodeAllocator, NodeAllocator, NodeAllocatorMap, NodeField, SimpleNodeAllocator,
    ZeroCopy, SENTINEL,
};
use crate::Container;
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
pub struct HashTableHeader<const NUM_BUCKETS: usize> {
    pub buckets: [u32; NUM_BUCKETS],
}

unsafe impl<const NUM_BUCKETS: usize> Zeroable for HashTableHeader<NUM_BUCKETS> {}
unsafe impl<const NUM_BUCKETS: usize> Pod for HashTableHeader<NUM_BUCKETS> {}
impl<const NUM_BUCKETS: usize> ZeroCopy for HashTableHeader<NUM_BUCKETS> {}
impl<const NUM_BUCKETS: usize> Default for HashTableHeader<NUM_BUCKETS> {
    fn default() -> Self {
        Self {
            buckets: [SENTINEL; NUM_BUCKETS],
        }
    }
}
impl<const NUM_BUCKETS: usize> AssertProperAlignment for HashTableHeader<NUM_BUCKETS> {
    fn assert_proper_alignment() {
        assert!(NUM_BUCKETS % 2 == 0);
    }
}

pub type HashTable<'a, K, V, Allocator, const NUM_BUCKETS: usize> =
    Container<'a, HashTableHeader<NUM_BUCKETS>, HashNode<K, V>, Allocator, 4>;
pub type StaticHashTable<'a, K, V, const NUM_BUCKETS: usize, const MAX_SIZE: usize> =
    HashTable<'a, K, V, SimpleNodeAllocator<HashNode<K, V>, MAX_SIZE, 4>, NUM_BUCKETS>;
pub type DynamicHashTable<'a, K, V, const NUM_BUCKETS: usize> =
    HashTable<'a, K, V, MultiArenaNodeAllocator<'a, HashNode<K, V>, 4>, NUM_BUCKETS>;

impl<
        'a,
        K: Hash + PartialEq + Copy + Clone + Default + Pod + Zeroable,
        V: Default + Copy + Clone + Pod + Zeroable,
        Allocator: NodeAllocator<HashNode<K, V>, 4>,
        const NUM_BUCKETS: usize,
    > NodeAllocatorMap<K, V> for HashTable<'a, K, V, Allocator, NUM_BUCKETS>
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

    fn len(&self) -> usize {
        self.allocator.size()
    }

    fn capacity(&self) -> usize {
        self.allocator.capacity()
    }

    fn iter(&self) -> Box<dyn DoubleEndedIterator<Item = (&K, &V)> + '_> {
        Box::new(self._iter())
    }

    fn iter_mut(&mut self) -> Box<dyn DoubleEndedIterator<Item = (&K, &mut V)> + '_> {
        // SAFETY: The trait requires lifetime '_ but we need to return references with lifetime 'a.
        // This is safe because 'a outlives the iterator lifetime.
        unsafe {
            let iter = self._iter_mut();
            std::mem::transmute::<
                Box<dyn DoubleEndedIterator<Item = (&K, &mut V)> + '_>,
                Box<dyn DoubleEndedIterator<Item = (&K, &mut V)> + '_>,
            >(Box::new(iter))
        }
    }
}

impl<
        'a,
        K: Hash + PartialEq + Copy + Clone + Default + Pod + Zeroable,
        V: Default + Copy + Clone + Pod + Zeroable,
        Allocator: NodeAllocator<HashNode<K, V>, 4>,
        const NUM_BUCKETS: usize,
    > HashTable<'a, K, V, Allocator, NUM_BUCKETS>
{
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

    fn _iter<'tree>(&'tree self) -> HashTableIterator<'tree, K, V, Allocator, NUM_BUCKETS>
    where
        'a: 'tree,
    {
        HashTableIterator::<K, V, Allocator, NUM_BUCKETS> {
            ht: self,
            bucket: 0,
            node: self.buckets[0],
        }
    }

    fn _iter_mut<'tree>(
        &'tree mut self,
    ) -> HashTableIteratorMut<'tree, 'a, K, V, Allocator, NUM_BUCKETS>
    where
        'a: 'tree,
    {
        let node = self.buckets[0];
        HashTableIteratorMut::<'tree, 'a, K, V, Allocator, NUM_BUCKETS> {
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
        Allocator: NodeAllocator<HashNode<K, V>, 4>,
        const NUM_BUCKETS: usize,
    > IntoIterator for &'a HashTable<'a, K, V, Allocator, NUM_BUCKETS>
{
    type Item = (&'a K, &'a V);
    type IntoIter = HashTableIterator<'a, K, V, Allocator, NUM_BUCKETS>;

    fn into_iter(self) -> Self::IntoIter {
        self._iter()
    }
}

impl<
        'a,
        K: Hash + PartialEq + Copy + Clone + Default + Pod + Zeroable,
        V: Default + Copy + Clone + Pod + Zeroable,
        Allocator: NodeAllocator<HashNode<K, V>, 4>,
        const NUM_BUCKETS: usize,
    > IntoIterator for &'a mut HashTable<'a, K, V, Allocator, NUM_BUCKETS>
{
    type Item = (&'a K, &'a mut V);
    type IntoIter = HashTableIteratorMut<'a, 'a, K, V, Allocator, NUM_BUCKETS>;

    fn into_iter(self) -> Self::IntoIter {
        self._iter_mut()
    }
}

pub struct HashTableIterator<
    'a,
    K: Hash + PartialEq + Copy + Clone + Default + Pod + Zeroable,
    V: Default + Copy + Clone + Pod + Zeroable,
    Allocator: NodeAllocator<HashNode<K, V>, 4>,
    const NUM_BUCKETS: usize,
> {
    ht: &'a HashTable<'a, K, V, Allocator, NUM_BUCKETS>,
    bucket: usize,
    node: u32,
}

impl<
        'a,
        K: Hash + PartialEq + Copy + Clone + Default + Pod + Zeroable,
        V: Default + Copy + Clone + Pod + Zeroable,
        Allocator: NodeAllocator<HashNode<K, V>, 4>,
        const NUM_BUCKETS: usize,
    > Iterator for HashTableIterator<'a, K, V, Allocator, NUM_BUCKETS>
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
        Allocator: NodeAllocator<HashNode<K, V>, 4>,
        const NUM_BUCKETS: usize,
    > DoubleEndedIterator for HashTableIterator<'a, K, V, Allocator, NUM_BUCKETS>
{
    fn next_back(&mut self) -> Option<Self::Item> {
        None
    }
}

pub struct HashTableIteratorMut<
    'tree,
    'a,
    K: Hash + PartialEq + Copy + Clone + Default + Pod + Zeroable,
    V: Default + Copy + Clone + Pod + Zeroable,
    Allocator: NodeAllocator<HashNode<K, V>, 4>,
    const NUM_BUCKETS: usize,
> where
    'a: 'tree,
{
    ht: &'tree mut HashTable<'a, K, V, Allocator, NUM_BUCKETS>,
    bucket: usize,
    node: u32,
}

impl<
        'tree,
        'a,
        K: Hash + PartialEq + Copy + Clone + Default + Pod + Zeroable,
        V: Default + Copy + Clone + Pod + Zeroable,
        Allocator: NodeAllocator<HashNode<K, V>, 4>,
        const NUM_BUCKETS: usize,
    > Iterator for HashTableIteratorMut<'tree, 'a, K, V, Allocator, NUM_BUCKETS>
where
    'a: 'tree,
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
            // SAFETY: This is required to extend the lifetime of the mutable reference
            // to 'a, but Rust's borrow checker cannot prove this is safe. The iterator
            // guarantees only one mutable reference to each node at a time, and the
            // iterator itself is unique, so this is sound as long as the iterator is
            // not misused (e.g., aliased or cloned).
            unsafe {
                let node: &mut HashNode<_, _> =
                    &mut *(&mut *self.ht.allocator.get_mut(ptr).get_value_mut() as *mut _);
                Some((&node.key, &mut node.value))
            }
        } else {
            None
        }
    }
}

impl<
        'tree,
        'a,
        K: Hash + PartialEq + Copy + Clone + Default + Pod + Zeroable,
        V: Default + Copy + Clone + Pod + Zeroable,
        Allocator: NodeAllocator<HashNode<K, V>, 4>,
        const NUM_BUCKETS: usize,
    > DoubleEndedIterator for HashTableIteratorMut<'tree, 'a, K, V, Allocator, NUM_BUCKETS>
where
    'a: 'tree,
{
    fn next_back(&mut self) -> Option<Self::Item> {
        None
    }
}

impl<
        'a,
        K: Hash + PartialEq + Copy + Clone + Default + Pod + Zeroable,
        V: Default + Copy + Clone + Pod + Zeroable,
        Allocator: NodeAllocator<HashNode<K, V>, 4>,
        const NUM_BUCKETS: usize,
    > Index<&K> for HashTable<'a, K, V, Allocator, NUM_BUCKETS>
{
    type Output = V;

    fn index(&self, index: &K) -> &Self::Output {
        self.get(index).unwrap()
    }
}

impl<
        'a,
        K: Hash + PartialEq + Copy + Clone + Default + Pod + Zeroable,
        V: Default + Copy + Clone + Pod + Zeroable,
        Allocator: NodeAllocator<HashNode<K, V>, 4>,
        const NUM_BUCKETS: usize,
    > IndexMut<&K> for HashTable<'a, K, V, Allocator, NUM_BUCKETS>
{
    fn index_mut(&mut self, index: &K) -> &mut Self::Output {
        self.get_mut(index).unwrap()
    }
}
