use bytemuck::{Pod, Zeroable};
use std::{
    collections::hash_map::DefaultHasher,
    hash::{Hash, Hasher},
};

use crate::node_allocator::{FromSlice, NodeAllocator, ZeroCopy, SENTINEL};

// The number of registers:
//   0 - bucket
//   1 - next pointer
//   2 and 3 - unused (needed for alignment)
const REGISTERS: usize = 4;

// Enum representing the registers (fields) of a node.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
enum Field {
    Next = 0,
    Bucket = 1,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct HashSet<
    V: Copy + Clone + Default + Hash + PartialEq + Pod + Zeroable,
    const MAX_SIZE: usize,
> {
    allocator: NodeAllocator<V, MAX_SIZE, REGISTERS>,
}

unsafe impl<V: Copy + Clone + Default + Hash + PartialEq + Pod + Zeroable, const MAX_SIZE: usize>
    Zeroable for HashSet<V, MAX_SIZE>
{
}
unsafe impl<V: Copy + Clone + Default + Hash + PartialEq + Pod + Zeroable, const MAX_SIZE: usize>
    Pod for HashSet<V, MAX_SIZE>
{
}

impl<V: Copy + Clone + Default + Hash + PartialEq + Pod + Zeroable, const MAX_SIZE: usize> ZeroCopy
    for HashSet<V, MAX_SIZE>
{
}

impl<V: Copy + Clone + Default + Hash + Pod + PartialEq + Zeroable, const MAX_SIZE: usize> FromSlice
    for HashSet<V, MAX_SIZE>
{
    fn new_from_slice(slice: &mut [u8]) -> &mut Self {
        let hash_set = Self::load_mut_bytes(slice).unwrap();
        hash_set.initialize();
        hash_set
    }
}

impl<V: Copy + Clone + Default + Hash + PartialEq + Pod + Zeroable, const MAX_SIZE: usize> Default
    for HashSet<V, MAX_SIZE>
{
    fn default() -> Self {
        HashSet {
            allocator: NodeAllocator::<V, MAX_SIZE, REGISTERS>::default(),
        }
    }
}

impl<V: Copy + Clone + Default + Hash + PartialEq + Pod + Zeroable, const MAX_SIZE: usize>
    HashSet<V, MAX_SIZE>
{
    pub fn new() -> Self {
        Self::default()
    }

    pub fn initialize(&mut self) {
        self.allocator.initialize()
    }

    pub fn len(&self) -> usize {
        self.allocator.size as usize
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn get_value(&self, node: u32) -> &V {
        self.allocator.get(node).get_value()
    }

    pub fn get_value_mut(&mut self, node: u32) -> &mut V {
        self.allocator.get_mut(node).get_value_mut()
    }

    pub fn insert(&mut self, value: V) -> bool {
        self._insert(value).is_some()
    }

    pub fn remove(&mut self, value: &V) -> bool {
        self._remove(value).is_some()
    }

    pub fn contains(&self, value: &V) -> bool {
        let bucket = Self::get_bucket(value);
        let head = self.get_field(bucket, Field::Bucket);

        let mut current = head;

        while current != SENTINEL {
            let node = self.allocator.get(current);
            if node.get_value() == value {
                return true;
            }

            current = self.get_field(current, Field::Next);
        }

        false
    }

    #[inline(always)]
    fn set_field(&mut self, node: u32, register: Field, value: u32) {
        self.allocator.set_register(node, value, register as u32);
    }

    #[inline(always)]
    fn get_field(&self, node: u32, register: Field) -> u32 {
        self.allocator.get_register(node, register as u32)
    }

    #[inline(always)]
    fn get_bucket(value: &V) -> u32 {
        let mut hasher = DefaultHasher::new();
        value.hash(&mut hasher);
        1 + (hasher.finish() as usize % MAX_SIZE) as u32
    }

    fn _insert(&mut self, value: V) -> Option<u32> {
        if self.allocator.size as usize == MAX_SIZE {
            return None;
        }

        let bucket = Self::get_bucket(&value);
        let head = self.get_field(bucket, Field::Bucket);
        let mut current = head;

        while current != SENTINEL {
            let node = self.allocator.get(current);
            // if the value is already present, we won't add
            // it again
            if node.get_value() == &value {
                return None;
            }

            current = self.get_field(current, Field::Next);
        }

        let node = self.allocator.add_node(value);
        self.set_field(bucket, Field::Bucket, node);
        self.set_field(node, Field::Next, head);

        Some(node)
    }

    fn _remove(&mut self, value: &V) -> Option<&V> {
        if self.allocator.size == 0 {
            return None;
        }

        let bucket = Self::get_bucket(value);
        let head = self.get_field(bucket, Field::Bucket);

        let mut current = head;
        let mut previous = SENTINEL;

        while current != SENTINEL {
            let node = self.allocator.get(current);
            let next = self.get_field(current, Field::Next);

            if node.get_value() == value {
                if previous == SENTINEL {
                    self.set_field(bucket, Field::Bucket, next);
                } else {
                    self.set_field(previous, Field::Next, next);
                }
                // clear the register 'next'
                self.set_field(current, Field::Next, SENTINEL);
                return self.allocator.remove_node(current);
            }

            previous = current;
            current = next;
        }

        None
    }

    pub fn iter(&self) -> HashSetIterator<'_, V, MAX_SIZE> {
        HashSetIterator::<V, MAX_SIZE> {
            hash_set: self,
            bucket: SENTINEL,
            node: SENTINEL,
        }
    }

    pub fn iter_mut(&mut self) -> HashSetIteratorMut<'_, V, MAX_SIZE> {
        HashSetIteratorMut::<V, MAX_SIZE> {
            hash_set: self,
            bucket: SENTINEL,
            node: SENTINEL,
        }
    }
}

pub struct HashSetIterator<
    'a,
    T: Copy + Clone + Default + Hash + PartialEq + Pod + Zeroable,
    const MAX_SIZE: usize,
> {
    hash_set: &'a HashSet<T, MAX_SIZE>,
    bucket: u32,
    node: u32,
}

impl<'a, T: Copy + Clone + Default + Hash + PartialEq + Pod + Zeroable, const MAX_SIZE: usize>
    Iterator for HashSetIterator<'a, T, MAX_SIZE>
{
    type Item = &'a T;

    fn next(&mut self) -> Option<Self::Item> {
        if self.bucket <= MAX_SIZE as u32 {
            while self.node == SENTINEL {
                self.bucket += 1;
                if self.bucket > MAX_SIZE as u32 {
                    return None;
                }
                self.node = self.hash_set.get_field(self.bucket, Field::Bucket);
            }
            let node = self.hash_set.get_value(self.node);
            self.node = self.hash_set.get_field(self.node, Field::Next);
            Some(node)
        } else {
            None
        }
    }
}

pub struct HashSetIteratorMut<
    'a,
    T: Copy + Clone + Default + Hash + PartialEq + Pod + Zeroable,
    const MAX_SIZE: usize,
> {
    hash_set: &'a mut HashSet<T, MAX_SIZE>,
    bucket: u32,
    node: u32,
}

impl<'a, T: Copy + Clone + Default + Hash + PartialEq + Pod + Zeroable, const MAX_SIZE: usize>
    Iterator for HashSetIteratorMut<'a, T, MAX_SIZE>
{
    type Item = &'a mut T;

    fn next(&mut self) -> Option<Self::Item> {
        if self.bucket <= MAX_SIZE as u32 {
            while self.node == SENTINEL {
                self.bucket += 1;
                if self.bucket > MAX_SIZE as u32 {
                    return None;
                }
                let head = self.hash_set.get_field(self.bucket, Field::Bucket);
                self.node = head;
            }
            let ptr = self.node;
            self.node = self.hash_set.get_field(self.node, Field::Next);
            unsafe {
                let node = (*self
                    .hash_set
                    .allocator
                    .nodes
                    .as_mut_ptr()
                    .add((ptr - 1) as usize))
                .get_value_mut();
                Some(node)
            }
        } else {
            None
        }
    }
}

#[test]
fn test_hash_set() {
    const CAPACITY: usize = 1024;
    type S = HashSet<u64, CAPACITY>;
    let mut buf = vec![0u8; std::mem::size_of::<S>()];
    let s = S::new_from_slice(buf.as_mut_slice());
    // insert
    (0..CAPACITY as u64).for_each(|v| {
        assert!(s.insert(v));
    });
    // contains
    (0..CAPACITY as u64).for_each(|v| {
        assert!(s.contains(&v));
    });
    // remove
    (0..CAPACITY as u64).for_each(|v| {
        assert!(s.remove(&v));
    });
    assert!(s.is_empty());
    // iter
    (0..CAPACITY as u64).for_each(|v| {
        assert!(s.insert(v));
    });
    assert_eq!(s.len(), CAPACITY);
    let values: Vec<u64> = s.iter().copied().collect();
    assert_eq!(values.len(), CAPACITY);
    values.iter().for_each(|v| {
        assert!(s.remove(v));
    });
    assert!(s.is_empty());
    values.iter().for_each(|v| {
        assert!(s.insert(*v));
    });
    assert_eq!(s.len(), CAPACITY);
}
