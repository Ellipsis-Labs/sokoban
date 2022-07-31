use bytemuck::{Pod, Zeroable};
use node_allocator::{NodeAllocator, ZeroCopy, SENTINEL};
use std::collections::hash_map::DefaultHasher;
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
    K: Hash + Copy + Clone + Default + Pod + Zeroable,
    V: Default + Copy + Clone + Pod + Zeroable,
> {
    pub key: K,
    pub value: V,
}

unsafe impl<
        K: Hash + Copy + Clone + Default + Pod + Zeroable,
        V: Default + Copy + Clone + Pod + Zeroable,
    > Zeroable for HashNode<K, V>
{
}
unsafe impl<
        K: Hash + Copy + Clone + Default + Pod + Zeroable,
        V: Default + Copy + Clone + Pod + Zeroable,
    > Pod for HashNode<K, V>
{
}

impl<
        K: Hash + Copy + Clone + Default + Pod + Zeroable,
        V: Default + Copy + Clone + Pod + Zeroable,
    > HashNode<K, V>
{
    pub fn new(key: K, value: V) -> Self {
        Self { key, value }
    }
}

#[derive(Copy, Clone)]
pub struct HashTable<
    K: Hash + Copy + Clone + Default + Pod + Zeroable,
    V: Default + Copy + Clone + Pod + Zeroable,
    const MAX_SIZE: usize,
> {
    pub sequence_number: u64,
    pub buckets: [u32; MAX_SIZE],
    pub allocator: NodeAllocator<HashNode<K, V>, MAX_SIZE, 2>,
}

unsafe impl<
        K: Hash + Copy + Clone + Default + Pod + Zeroable,
        V: Default + Copy + Clone + Pod + Zeroable,
        const MAX_SIZE: usize,
    > Zeroable for HashTable<K, V, MAX_SIZE>
{
}
unsafe impl<
        K: Hash + Copy + Clone + Default + Pod + Zeroable,
        V: Default + Copy + Clone + Pod + Zeroable,
        const MAX_SIZE: usize,
    > Pod for HashTable<K, V, MAX_SIZE>
{
}

impl<
        K: Hash + Copy + Clone + Default + Pod + Zeroable,
        V: Default + Copy + Clone + Pod + Zeroable,
        const MAX_SIZE: usize,
    > ZeroCopy for HashTable<K, V, MAX_SIZE>
{
}

impl<
        K: Hash + Copy + Clone + Default + Pod + Zeroable,
        V: Default + Copy + Clone + Pod + Zeroable,
        const MAX_SIZE: usize,
    > Default for HashTable<K, V, MAX_SIZE>
{
    fn default() -> Self {
        HashTable {
            sequence_number: 0,
            buckets: [SENTINEL; MAX_SIZE],
            allocator: NodeAllocator::<HashNode<K, V>, MAX_SIZE, 2>::default(),
        }
    }
}

impl<
        K: Hash + Copy + Clone + Default + Pod + Zeroable,
        V: Default + Copy + Clone + Pod + Zeroable,
        const MAX_SIZE: usize,
    > HashTable<K, V, MAX_SIZE>
{
}

// impl<
//         const MAX_SIZE: usize,
//         K: Hash + Copy + Clone + Default + Pod + Zeroable,
//         V: Default + Copy + Clone + Pod + Zeroable,
//     > Index<&K> for HashTable<MAX_SIZE, K, V>
// {
//     type Output = V;

//     fn index(&self, index: &K) -> &Self::Output {
//         &self.get(index).unwrap()
//     }
// }

// impl<
//         const MAX_SIZE: usize,
//         K: Hash + Copy + Clone + Default + Pod + Zeroable,
//         V: Default + Copy + Clone + Pod + Zeroable,
//     > IndexMut<&K> for HashTable<MAX_SIZE, K, V>
// {
//     fn index_mut(&mut self, index: &K) -> &mut Self::Output {
//         self.get_mut(index).unwrap()
//     }
// }
