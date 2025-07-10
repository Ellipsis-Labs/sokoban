use bytemuck::{Pod, Zeroable};
use std::mem::{align_of, size_of};

use crate::ZeroCopy;

use super::{Node, NodeAllocator, SENTINEL};

/// Returns the number of nodes in an arena
pub const fn block_size_of<
    T: Default + Copy + Clone + Pod + Zeroable,
    const NUM_REGISTERS: usize,
    const MAX_SIZE: usize,
>() -> usize {
    MAX_SIZE / std::mem::size_of::<Node<T, NUM_REGISTERS>>()
}
/**
 * Superblock consists of the header and a vector containing the size of each arena
 * The reader is a proxy type for zero-copy deserialization of the superblock
 */

#[repr(C)]
#[derive(Clone, Copy, std::fmt::Debug)]
pub struct Superblock {
    /// Size of the allocator. The max value this can take is `max_size`
    pub size: u64,

    /// The max size of the allocator.
    pub max_size: u64,

    /// Number of arenas in the superblock, should matches the number of entries in real allocator
    pub num_arenas: u32,

    /// Index that represents the "boundary" of the allocator. When this value reaches `MAX_SIZE`
    /// this indicates that all of the nodes has been used at least once and all new allocated
    /// indicies must be pulled from the free list.
    /// Note: the index here is the logical page number
    bump_index: u32,
    /// Buffer index of the first element in the free list. The free list is a singly-linked list
    /// of unallocated nodes. The free list operates like a stack. When a node is removed from the
    /// allocator, the removed node becomes the new free list head. When new nodes are added,
    /// the new index to allocated is pulled from the `free_list_head`
    /// Note: the index here is the logical page number
    free_list_head: u32,
}

unsafe impl Zeroable for Superblock {}
unsafe impl Pod for Superblock {}
impl ZeroCopy for Superblock {}

impl Superblock {
    fn initialize(&mut self, num_arenas: usize, max_size: usize) {
        if self.size == 0
            && self.num_arenas == 0
            && self.bump_index == 0
            && self.free_list_head == 0
            && self.max_size == 0
        {
            self.max_size = max_size as u64;
            self.num_arenas = num_arenas as u32;
            self.bump_index = 1;
            self.free_list_head = 1;
        } else {
            panic!("Cannot reinitialize NodeAllocator");
        }
    }
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct Arena<
    T: Default + Copy + Clone + Pod + Zeroable,
    const BLOCK_SIZE: usize,
    const NUM_REGISTERS: usize,
> {
    /// Nodes containing data, with `NUM_REGISTERS` registers that store arbitrary data
    pub nodes: [Node<T, NUM_REGISTERS>; BLOCK_SIZE],
}

unsafe impl<
        T: Default + Copy + Clone + Pod + Zeroable,
        const BLOCK_SIZE: usize,
        const NUM_REGISTERS: usize,
    > Zeroable for Arena<T, BLOCK_SIZE, NUM_REGISTERS>
{
}
unsafe impl<
        T: Default + Copy + Clone + Pod + Zeroable,
        const BLOCK_SIZE: usize,
        const NUM_REGISTERS: usize,
    > Pod for Arena<T, BLOCK_SIZE, NUM_REGISTERS>
{
}

impl<
        T: Default + Copy + Clone + Pod + Zeroable,
        const BLOCK_SIZE: usize,
        const NUM_REGISTERS: usize,
    > ZeroCopy for Arena<T, BLOCK_SIZE, NUM_REGISTERS>
{
}

impl<
        T: Default + Copy + Clone + Pod + Zeroable,
        const BLOCK_SIZE: usize,
        const NUM_REGISTERS: usize,
    > Default for Arena<T, BLOCK_SIZE, NUM_REGISTERS>
{
    fn default() -> Self {
        Self {
            nodes: [Node::<T, NUM_REGISTERS>::default(); BLOCK_SIZE],
        }
    }
}
impl<
        T: Default + Copy + Clone + Pod + Zeroable,
        const BLOCK_SIZE: usize,
        const NUM_REGISTERS: usize,
    > Arena<T, BLOCK_SIZE, NUM_REGISTERS>
{
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
        assert!(t_index % t_align == 0, "First index of T is misaligned");
        assert!(
            (t_index + t_size + reg_size) % t_align == 0,
            "Subsequent indices of T are misaligned"
        );
    }
}

pub struct MultiArenaNodeAllocator<
    'a,
    T: Default + Copy + Clone + Pod + Zeroable,
    const BLOCK_SIZE: usize,
    const NUM_REGISTERS: usize,
> {
    pub superblock: &'a mut Superblock,
    pub arenas: Vec<&'a mut Arena<T, BLOCK_SIZE, NUM_REGISTERS>>,
}

impl<
        'a,
        T: Default + Copy + Clone + Pod + Zeroable,
        const BLOCK_SIZE: usize,
        const NUM_REGISTERS: usize,
    > MultiArenaNodeAllocator<'a, T, BLOCK_SIZE, NUM_REGISTERS>
{
    fn new(
        superblock: &'a mut Superblock,
        arenas: Vec<&'a mut Arena<T, BLOCK_SIZE, NUM_REGISTERS>>,
    ) -> Self {
        Self { superblock, arenas }
    }

    #[inline(always)]
    fn index_conv(&self, i: u32) -> (usize, usize) {
        let page_no = i - 1;
        let block_no = page_no / BLOCK_SIZE as u32;
        let node_no = page_no % BLOCK_SIZE as u32;
        (block_no as usize, node_no as usize)
    }

    pub fn from_buffers(
        superblock_buffer: &'a mut [u8],
        arena_buffers: &'a mut [&'a mut [u8]],
    ) -> Self {
        let superblock = Superblock::load_mut_bytes(superblock_buffer)
            .expect("Failed to load Superblock from buffer");
        let mut arenas = Vec::with_capacity(arena_buffers.len());
        for arena_buffer in arena_buffers.iter_mut() {
            let arena =
                Arena::load_mut_bytes(*arena_buffer).expect("Failed to load Arena from buffer");
            arenas.push(arena);
        }
        Self::new(superblock, arenas)
    }
}

impl<
        'a,
        T: Default + Copy + Clone + Pod + Zeroable,
        const BLOCK_SIZE: usize,
        const NUM_REGISTERS: usize,
    > NodeAllocator<T, NUM_REGISTERS>
    for MultiArenaNodeAllocator<'a, T, BLOCK_SIZE, NUM_REGISTERS>
{
    #[inline(always)]
    fn assert_proper_alignment(&self) {
        self.arenas.iter().for_each(|arena| {
            arena.assert_proper_alignment();
        });
    }

    fn size(&self) -> usize {
        self.superblock.size as usize
    }

    fn initialize(&mut self, max_size: usize) {
        assert!(NUM_REGISTERS >= 1);
        self.assert_proper_alignment();
        self.superblock.initialize(self.arenas.len(), max_size);
    }

    #[inline(always)]
    fn get(&self, i: u32) -> &Node<T, NUM_REGISTERS> {
        let (arena_index, node_index) = self.index_conv(i);
        &self.arenas[arena_index].nodes[node_index]
    }

    #[inline(always)]
    fn get_mut(&mut self, i: u32) -> &mut Node<T, NUM_REGISTERS> {
        let (arena_index, node_index) = self.index_conv(i);
        &mut self.arenas[arena_index].nodes[node_index]
    }

    /// Adds a new node to the allocator. The function returns the current pointer
    /// to the free list, where the new node is inserted
    fn add_node(&mut self, node: T) -> u32 {
        let i = self.superblock.free_list_head;
        if self.superblock.free_list_head == self.superblock.bump_index {
            if self.superblock.bump_index
                == (self.superblock.num_arenas * BLOCK_SIZE as u32 + 1) as u32
            {
                panic!(
                    "Buffer is full, size {}, time to add a new arena",
                    self.superblock.size
                );
            }
            self.superblock.bump_index += 1;
            self.superblock.free_list_head = self.superblock.bump_index;
        } else {
            self.superblock.free_list_head = self.get(i).get_free_list_register();
            self.get_mut(i).set_free_list_register(SENTINEL);
        }
        self.get_mut(i).set_value(node);
        self.superblock.size += 1;
        i
    }

    /// Removes the node at index `i` from the allocator and adds the index to the free list
    /// When deleting nodes, you MUST clear all registers prior to calling `remove_node`
    fn remove_node(&mut self, i: u32) -> Option<&T> {
        if i == SENTINEL {
            return None;
        }
        let free_list_head = self.superblock.free_list_head;
        self.get_mut(i).set_free_list_register(free_list_head);
        self.superblock.free_list_head = i;
        self.superblock.size -= 1;
        Some(self.get(i).get_value())
    }
}
