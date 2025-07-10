use bytemuck::{Pod, Zeroable};
use std::mem::{align_of, size_of};

use super::{Node, NodeAllocator, ZeroCopy, SENTINEL};

#[repr(C)]
#[derive(Copy, Clone)]
pub struct SimpleNodeAllocator<
    T: Default + Copy + Clone + Pod + Zeroable,
    const MAX_SIZE: usize,
    const NUM_REGISTERS: usize,
> {
    /// Size of the allocator. The max value this can take is `MAX_SIZE`
    pub size: u64,
    /// Index that represents the "boundary" of the allocator. When this value reaches `MAX_SIZE`
    /// this indicates that all of the nodes has been used at least once and all new allocated
    /// indicies must be pulled from the free list.
    bump_index: u32,
    /// Buffer index of the first element in the free list. The free list is a singly-linked list
    /// of unallocated nodes. The free list operates like a stack. When a node is removed from the
    /// allocator, the removed node becomes the new free list head. When new nodes are added,
    /// the new index to allocated is pulled from the `free_list_head`
    free_list_head: u32,
    /// Nodes containing data, with `NUM_REGISTERS` registers that store arbitrary data
    pub nodes: [Node<T, NUM_REGISTERS>; MAX_SIZE],
}

unsafe impl<
        T: Default + Copy + Clone + Pod + Zeroable,
        const MAX_SIZE: usize,
        const NUM_REGISTERS: usize,
    > Zeroable for SimpleNodeAllocator<T, MAX_SIZE, NUM_REGISTERS>
{
}
unsafe impl<
        T: Default + Copy + Clone + Pod + Zeroable,
        const MAX_SIZE: usize,
        const NUM_REGISTERS: usize,
    > Pod for SimpleNodeAllocator<T, MAX_SIZE, NUM_REGISTERS>
{
}

impl<
        T: Default + Copy + Clone + Pod + Zeroable,
        const MAX_SIZE: usize,
        const NUM_REGISTERS: usize,
    > ZeroCopy for SimpleNodeAllocator<T, MAX_SIZE, NUM_REGISTERS>
{
}

impl<
        T: Default + Copy + Clone + Pod + Zeroable,
        const MAX_SIZE: usize,
        const NUM_REGISTERS: usize,
    > Default for SimpleNodeAllocator<T, MAX_SIZE, NUM_REGISTERS>
{
    fn default() -> Self {
        assert!(NUM_REGISTERS >= 1);
        let na = SimpleNodeAllocator {
            size: 0,
            bump_index: 1,
            free_list_head: 1,
            nodes: [Node::<T, NUM_REGISTERS>::default(); MAX_SIZE],
        };
        na.assert_proper_alignment();
        na
    }
}

impl<
        T: Default + Copy + Clone + Pod + Zeroable,
        const MAX_SIZE: usize,
        const NUM_REGISTERS: usize,
    > SimpleNodeAllocator<T, MAX_SIZE, NUM_REGISTERS>
{
    pub fn new() -> Self {
        Self::default()
    }
}

impl<
        T: Default + Copy + Clone + Pod + Zeroable,
        const MAX_SIZE: usize,
        const NUM_REGISTERS: usize,
    > NodeAllocator<T, NUM_REGISTERS> for SimpleNodeAllocator<T, MAX_SIZE, NUM_REGISTERS>
{
    #[inline(always)]
    fn assert_proper_alignment(&self) {
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

    fn initialize(&mut self) {
        assert!(NUM_REGISTERS >= 1);
        self.assert_proper_alignment();
        if self.size == 0 && self.bump_index == 0 && self.free_list_head == 0 {
            self.bump_index = 1;
            self.free_list_head = 1;
        } else {
            panic!("Cannot reinitialize NodeAllocator");
        }
    }

    fn size(&self) -> usize {
        self.size as usize
    }

    #[inline(always)]
    fn get(&self, i: u32) -> &Node<T, NUM_REGISTERS> {
        &self.nodes[(i - 1) as usize]
    }

    #[inline(always)]
    fn get_mut(&mut self, i: u32) -> &mut Node<T, NUM_REGISTERS> {
        &mut self.nodes[(i - 1) as usize]
    }

    /// Adds a new node to the allocator. The function returns the current pointer
    /// to the free list, where the new node is inserted
    fn add_node(&mut self, node: T) -> u32 {
        let i = self.free_list_head;
        if self.free_list_head == self.bump_index {
            if self.bump_index == (MAX_SIZE + 1) as u32 {
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

    /// Removes the node at index `i` from the allocator and adds the index to the free list
    /// When deleting nodes, you MUST clear all registers prior to calling `remove_node`
    fn remove_node(&mut self, i: u32) -> Option<&T> {
        if i == SENTINEL {
            return None;
        }
        let free_list_head = self.free_list_head;
        self.get_mut(i).set_free_list_register(free_list_head);
        self.free_list_head = i;
        self.size -= 1;
        Some(self.get(i).get_value())
    }
}
