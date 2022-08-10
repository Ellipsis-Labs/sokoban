use bytemuck::{Pod, Zeroable};
use std::mem::{align_of, size_of};
use num_derive::FromPrimitive;

// Enum representing the fields of a tree node:
// 0 - left pointer
// 1 - right pointer
// 2 - parent pointer 
// 3 - value pointer (index of leaf) 
#[derive(Debug, Copy, Clone, PartialEq, FromPrimitive)]
pub enum TreeField {
    Left = 0,
    Right = 1,
    Parent = 2,
    Value = 3,
}

// Enum representing the fields of a simple node (Linked List / Binary Tree):
// 0 - left pointer
// 1 - right pointer
#[derive(Debug, Copy, Clone, PartialEq, FromPrimitive)]
pub enum NodeField {
    Left = 0,
    Right = 1,
}

pub trait FromSlice {
    fn new_from_slice(data: &mut [u8]) -> &mut Self;
}

pub trait NodeAllocatorMap<K, V> {
    fn insert(&mut self, key: K, value: V) -> Option<u32>;
    fn remove(&mut self, key: &K) -> Option<V>;
    fn size(&self) -> usize; 
    fn iter(&self) -> Box<dyn Iterator<Item = (&K, &V)> + '_>;
    fn iter_mut(&mut self) -> Box<dyn Iterator<Item = (&K, &mut V)> + '_>;
}

pub trait ZeroCopy: Pod {
    fn load_mut_bytes<'a>(data: &'a mut [u8]) -> Option<&'a mut Self> {
        let size = std::mem::size_of::<Self>();
        bytemuck::try_from_bytes_mut(&mut data[..size]).ok()
    }

    fn load_bytes<'a>(data: &'a [u8]) -> Option<&'a Self> {
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

#[repr(C)]
#[derive(Copy, Clone)]
pub struct NodeAllocator<
    T: Default + Copy + Clone + Pod + Zeroable,
    const MAX_SIZE: usize,
    const NUM_REGISTERS: usize,
> {
    /// Size of the allocator. The max value this can take is `MAX_SIZE - 1` because the 0-index is always
    /// the SENTINEL
    pub size: u64,
    /// Furthest index of the allocator. When this value reaches `MAX_SIZE` this indicates taht all of the nodes
    /// has been used at least once and new allocated indicies must be pulled from the free list.
    bump_index: u32,
    /// Buffer index of the first element in the free list. The free list is a linked list of nodes that
    /// unallocated. The free list operates like a stack. When nodes are removed from the allocator,
    /// that node becomes the new free list head. When new nodes are added, the index is pull from the
    /// `free_list_head`
    free_list_head: u32,
    /// Nodes containing data, with `NUM_REGISTERS` registers that can store arbitrary data  
    pub nodes: [Node<T, NUM_REGISTERS>; MAX_SIZE],
}

unsafe impl<
        T: Default + Copy + Clone + Pod + Zeroable,
        const MAX_SIZE: usize,
        const NUM_REGISTERS: usize,
    > Zeroable for NodeAllocator<T, MAX_SIZE, NUM_REGISTERS>
{
}
unsafe impl<
        T: Default + Copy + Clone + Pod + Zeroable,
        const MAX_SIZE: usize,
        const NUM_REGISTERS: usize,
    > Pod for NodeAllocator<T, MAX_SIZE, NUM_REGISTERS>
{
}

impl<
        T: Default + Copy + Clone + Pod + Zeroable,
        const MAX_SIZE: usize,
        const NUM_REGISTERS: usize,
    > ZeroCopy for NodeAllocator<T, MAX_SIZE, NUM_REGISTERS>
{
}

impl<
        T: Default + Copy + Clone + Pod + Zeroable,
        const MAX_SIZE: usize,
        const NUM_REGISTERS: usize,
    > Default for NodeAllocator<T, MAX_SIZE, NUM_REGISTERS>
{
    fn default() -> Self {
        assert!(NUM_REGISTERS >= 1);
        let na = NodeAllocator {
            size: 0,
            bump_index: 1,
            free_list_head: 1,
            nodes: [Node::<T, NUM_REGISTERS>::default(); MAX_SIZE],
        };
        na.assert_proper_alignemnt();
        na
    }
}

impl<
        T: Default + Copy + Clone + Pod + Zeroable,
        const MAX_SIZE: usize,
        const NUM_REGISTERS: usize,
    > NodeAllocator<T, MAX_SIZE, NUM_REGISTERS>
{
    pub fn new() -> Self {
        Self::default()
    }

    #[inline(always)]
    fn assert_proper_alignemnt(&self) {
        let reg_size = size_of::<u32>() * NUM_REGISTERS;
        let self_ptr = std::slice::from_ref(self).as_ptr() as usize;
        let node_ptr = std::slice::from_ref(&self.nodes).as_ptr() as usize;
        let self_align = align_of::<Self>();
        let t_index = node_ptr + reg_size;
        let t_align = align_of::<T>();
        let t_size = size_of::<T>();
        assert!(
            self_ptr % self_align as usize == 0,
            "NodeAllocator alignment mismatch, address is {} which is not a multiple of the struct alignment ({})",
            self_ptr,
            self_align,
        );
        assert!(
            t_size % t_align == 0,
            "Size of T ({}) is not a multiple of the alignment of T ({})",
            t_size,
            t_align,
        );
        assert!(
            t_size == 0 || t_size >= self_align,
            "Size of T ({}) must be >= than the alignment of NodeAllocator ({})",
            t_size,
            self_align,
        );
        assert!(node_ptr == self_ptr + 16, "Nodes are misaligned");
        assert!(t_index % t_align == 0, "First index of T is misaligned");
        assert!(
            (t_index + t_size + reg_size) % t_align == 0,
            "Subsequent indices of T are misaligned"
        );
    }

    pub fn initialize(&mut self) {
        assert!(NUM_REGISTERS >= 1);
        self.assert_proper_alignemnt();
        if self.size == 0 && self.bump_index == 0 && self.free_list_head == 0 {
            self.bump_index = 1;
            self.free_list_head = 1;
        } else {
            panic!("Cannot reinitialize NodeAllocator");
        }
    }

    #[inline(always)]
    pub fn get(&self, i: u32) -> &Node<T, NUM_REGISTERS> {
        &self.nodes[i as usize]
    }

    #[inline(always)]
    pub fn get_mut(&mut self, i: u32) -> &mut Node<T, NUM_REGISTERS> {
        &mut self.nodes[i as usize]
    }

    /// Adds a new node to the allocator. The function returns the current pointer
    /// to the free list, where the new node is inserted
    pub fn add_node(&mut self, node: T) -> u32 {
        let i = self.free_list_head;
        if self.free_list_head == self.bump_index {
            if self.bump_index == MAX_SIZE as u32 {
                panic!("Buffer is full, size {}", self.size);
            }
            self.bump_index += 1;
            self.free_list_head = self.bump_index;
        } else {
            self.free_list_head = self.get(i).get_free_list_register();
            self.get_mut(i).set_free_list_register(SENTINEL);
        }
        self.get_mut(i).set_value(node);
        self.size += 1;
        i
    }

    /// Removes the node at index `i` from the alloctor and adds the index to the free list
    /// When deleting nodes, you MUST clear all registers prior to calling `remove_node`
    pub fn remove_node(&mut self, i: u32) -> Option<&T> {
        if i == SENTINEL {
            return None;
        }
        let free_list_head = self.free_list_head;
        self.get_mut(i).set_free_list_register(free_list_head);
        self.free_list_head = i;
        self.size -= 1;
        Some(self.get(i).get_value())
    }

    #[inline(always)]
    pub fn disconnect(&mut self, i: u32, j: u32, r_i: u32, r_j: u32) {
        if i != SENTINEL {
            assert!(j == self.get_register(i, r_i), "Nodes are not connected");
            self.clear_register(i, r_i);
        }
        if j != SENTINEL {
            assert!(i == self.get_register(j, r_j), "Nodes are not connected");
            self.clear_register(j, r_j);
        }
    }

    #[inline(always)]
    pub fn clear_register(&mut self, i: u32, r_i: u32) {
        self.get_mut(i).set_register(r_i as usize, SENTINEL);
    }

    #[inline(always)]
    pub fn connect(&mut self, i: u32, j: u32, r_i: u32, r_j: u32) {
        if i != SENTINEL {
            self.get_mut(i).set_register(r_i as usize, j);
        }
        if j != SENTINEL {
            self.get_mut(j).set_register(r_j as usize, i);
        }
    }

    #[inline(always)]
    pub fn set_register(&mut self, i: u32, value: u32, r_i: u32) {
        if i != SENTINEL {
            self.get_mut(i).set_register(r_i as usize, value);
        }
    }

    #[inline(always)]
    pub fn get_register(&self, i: u32, r_i: u32) -> u32 {
        self.get(i).get_register(r_i as usize)
    }
}
