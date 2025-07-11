mod helper;
mod multi_arena;
mod simple;

use bytemuck::{Pod, Zeroable};

pub use helper::*;
pub use multi_arena::{
    max_number_of_nodes_in_block, size_of_nodes, MultiArenaNodeAllocator, Superblock,
};
pub use simple::SimpleNodeAllocator;

/// This is a convenience trait that exposes an interface to read a struct from an arbitrary byte array
pub trait FromSlice {
    fn new_from_slice(data: &mut [u8]) -> &mut Self;
}

/// This trait provides an API for map-like data structures that use the NodeAllocator
/// struct as the underlying container
pub trait NodeAllocatorMap<K, V> {
    fn insert(&mut self, key: K, value: V) -> Option<u32>;
    fn remove(&mut self, key: &K) -> Option<V>;
    fn contains(&self, key: &K) -> bool;
    fn get(&self, key: &K) -> Option<&V>;
    fn get_mut(&mut self, key: &K) -> Option<&mut V>;
    #[deprecated]
    fn size(&self) -> usize;
    fn len(&self) -> usize;
    fn is_empty(&self) -> bool {
        self.len() == 0
    }
    fn capacity(&self) -> usize;
    fn iter(&self) -> Box<dyn DoubleEndedIterator<Item = (&K, &V)> + '_>;
    fn iter_mut(&mut self) -> Box<dyn DoubleEndedIterator<Item = (&K, &mut V)> + '_>;
}

/// This trait adds additional functions for sorted map data structures that use the NodeAllocator
pub trait OrderedNodeAllocatorMap<K, V>: NodeAllocatorMap<K, V> {
    fn get_min_index(&mut self) -> u32;
    fn get_max_index(&mut self) -> u32;
    fn get_min(&mut self) -> Option<(K, V)>;
    fn get_max(&mut self) -> Option<(K, V)>;
}

pub trait ZeroCopy: Pod {
    fn load_mut_bytes(data: &'_ mut [u8]) -> Option<&'_ mut Self> {
        let size = std::mem::size_of::<Self>();
        bytemuck::try_from_bytes_mut(&mut data[..size]).ok()
    }

    fn load_bytes(data: &'_ [u8]) -> Option<&'_ Self> {
        let size = std::mem::size_of::<Self>();
        bytemuck::try_from_bytes(&data[..size]).ok()
    }
}

pub const SENTINEL: u32 = 0;

#[repr(C)]
#[derive(Copy, Clone)]
pub struct Node<T: Copy + Clone + Pod + Zeroable + Default, const NUM_REGISTERS: usize> {
    /// Arbitrary registers (generally used for pointers)
    /// Note: Register 0 is ALWAYS used for the free list
    registers: [u32; NUM_REGISTERS],
    value: T,
}

impl<T: Copy + Clone + Pod + Zeroable + Default, const NUM_REGISTERS: usize> Default
    for Node<T, NUM_REGISTERS>
{
    fn default() -> Self {
        assert!(NUM_REGISTERS >= 1);
        Self {
            registers: [SENTINEL; NUM_REGISTERS],
            value: T::default(),
        }
    }
}

unsafe impl<T: Copy + Clone + Pod + Zeroable + Default, const NUM_REGISTERS: usize> Zeroable
    for Node<T, NUM_REGISTERS>
{
}
unsafe impl<T: Copy + Clone + Pod + Zeroable + Default, const NUM_REGISTERS: usize> Pod
    for Node<T, NUM_REGISTERS>
{
}

impl<T: Copy + Clone + Pod + Zeroable + Default, const NUM_REGISTERS: usize> ZeroCopy
    for Node<T, NUM_REGISTERS>
{
}

impl<T: Copy + Clone + Pod + Zeroable + Default, const NUM_REGISTERS: usize>
    Node<T, NUM_REGISTERS>
{
    #[inline(always)]
    pub(crate) fn get_free_list_register(&self) -> u32 {
        self.registers[0]
    }

    #[inline(always)]
    pub fn get_register(&self, r: usize) -> u32 {
        self.registers[r]
    }

    #[inline(always)]
    pub(crate) fn set_free_list_register(&mut self, v: u32) {
        self.registers[0] = v;
    }

    #[inline(always)]
    pub fn set_register(&mut self, r: usize, v: u32) {
        self.registers[r] = v;
    }

    #[inline(always)]
    pub fn set_value(&mut self, v: T) {
        self.value = v;
    }

    #[inline(always)]
    pub fn get_value_mut(&mut self) -> &mut T {
        &mut self.value
    }

    #[inline(always)]
    pub fn get_value(&self) -> &T {
        &self.value
    }
}

pub trait NodeAllocator<T: Copy + Clone + Pod + Zeroable + Default, const NUM_REGISTERS: usize> {
    fn assert_proper_alignment(&self);

    fn initialize(&mut self);

    fn get(&self, i: u32) -> &Node<T, NUM_REGISTERS>;

    fn get_mut(&mut self, i: u32) -> &mut Node<T, NUM_REGISTERS>;

    fn add_node(&mut self, node: T) -> u32;

    fn remove_node(&mut self, i: u32) -> Option<&T>;

    fn size(&self) -> usize;

    fn capacity(&self) -> usize;

    #[inline(always)]
    fn get_register(&self, i: u32, r_i: u32) -> u32 {
        if i != SENTINEL {
            self.get(i).get_register(r_i as usize)
        } else {
            SENTINEL
        }
    }

    #[inline(always)]
    fn set_register(&mut self, i: u32, value: u32, r_i: u32) {
        if i != SENTINEL {
            self.get_mut(i).set_register(r_i as usize, value);
        }
    }

    #[inline(always)]
    fn clear_register(&mut self, i: u32, r_i: u32) {
        if i != SENTINEL {
            self.get_mut(i).set_register(r_i as usize, SENTINEL);
        }
    }

    #[inline(always)]
    fn connect(&mut self, i: u32, j: u32, r_i: u32, r_j: u32) {
        if i != SENTINEL {
            self.get_mut(i).set_register(r_i as usize, j);
        }
        if j != SENTINEL {
            self.get_mut(j).set_register(r_j as usize, i);
        }
    }

    #[inline(always)]
    fn disconnect(&mut self, i: u32, j: u32, r_i: u32, r_j: u32) {
        if i != SENTINEL {
            // assert!(j == self.get_register(i, r_i), "Nodes are not connected");
            self.clear_register(i, r_i);
        }
        if j != SENTINEL {
            // assert!(i == self.get_register(j, r_j), "Nodes are not connected");
            self.clear_register(j, r_j);
        }
    }
}
