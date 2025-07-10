use bytemuck::{Pod, Zeroable};
use std::mem::{align_of, size_of};

use crate::ZeroCopy;

use super::{Node, NodeAllocator, SENTINEL};

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

    // we no longer do a free list, instead we swap the nodes to the tail, pop it and reduce the bump_index
    // This records the index of the last removed node, which is used to swap the node to the tail
    // If it's not SENTINEL, the data structure is in a dirty state
    // and you have to call `start_pop_up` and `finish_pop_up` correctly to fix the pointers
    last_removed_node: u32,
}

unsafe impl Zeroable for Superblock {}
unsafe impl Pod for Superblock {}
impl ZeroCopy for Superblock {}

impl Superblock {
    fn initialize(&mut self, num_arenas: usize, max_size: usize) {
        if self.size == 0 && self.num_arenas == 0 && self.bump_index == 0 && self.max_size == 0 {
            self.max_size = max_size as u64;
            self.num_arenas = num_arenas as u32;
            self.bump_index = 1;
        } else {
            panic!("Cannot reinitialize NodeAllocator");
        }
    }
}

/// Swap and pop allocator
pub struct MultiArenaNodeAllocator<
    'a,
    T: Default + Copy + Clone + Pod + Zeroable,
    const BLOCK_SIZE: usize,
    const NUM_REGISTERS: usize,
> {
    pub superblock: &'a mut Superblock,
    pub arenas: Vec<&'a mut [Node<T, NUM_REGISTERS>]>,
}

impl<
        'a,
        T: Default + Copy + Clone + Pod + Zeroable,
        const BLOCK_SIZE: usize,
        const NUM_REGISTERS: usize,
    > MultiArenaNodeAllocator<'a, T, BLOCK_SIZE, NUM_REGISTERS>
{
    fn new(superblock: &'a mut Superblock, arenas: Vec<&'a mut [Node<T, NUM_REGISTERS>]>) -> Self {
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
            let nodes = bytemuck::cast_slice_mut::<u8, Node<T, NUM_REGISTERS>>(arena_buffer);
            arenas.push(nodes);
        }
        Self::new(superblock, arenas)
    }

    /// Swapping the last node with the last_removed_node, returns the index of last removed node and current bump index
    /// The upstream code should manipulate the pointers manually to make the structure consistent
    pub fn start_pop_up(&mut self) -> Option<(u32, u32)> {
        if self.superblock.last_removed_node == SENTINEL {
            return None;
        }
        let last_node = *self.get(self.superblock.bump_index - 1);
        *self.get_mut(self.superblock.last_removed_node) = last_node;
        Some((
            self.superblock.last_removed_node,
            self.superblock.bump_index - 1,
        ))
    }

    // call this once the last removed node must be correctly swapped out
    pub fn finish_pop_up(&mut self) {
        self.superblock.last_removed_node = SENTINEL;
        self.superblock.bump_index -= 1;
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
        let reg_size = size_of::<u32>() * NUM_REGISTERS;
        let self_ptr = std::slice::from_ref(self).as_ptr() as usize;
        let self_align = align_of::<Self>();
        for arena in self.arenas.iter() {
            let node_ptr = (*arena).as_ptr() as usize;
            let t_index = node_ptr + reg_size;
            let t_align = align_of::<T>();
            let t_size = size_of::<T>();
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
        assert!(
            self_ptr % self_align as usize == 0,
            "NodeAllocator alignment mismatch, address is {} which is not a multiple of the struct alignment ({})",
            self_ptr,
            self_align,
        );
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
        &self.arenas[arena_index][node_index]
    }

    #[inline(always)]
    fn get_mut(&mut self, i: u32) -> &mut Node<T, NUM_REGISTERS> {
        let (arena_index, node_index) = self.index_conv(i);
        &mut self.arenas[arena_index][node_index]
    }

    /// Adds a new node to the allocator. The function returns the current pointer
    /// to the free list, where the new node is inserted
    fn add_node(&mut self, node: T) -> u32 {
        if self.superblock.bump_index == self.superblock.max_size as u32 + 1 {
            panic!(
                "Buffer is full, size {}, time to add a new arena",
                self.superblock.size
            );
        }
        let i = self.superblock.bump_index;
        self.get_mut(i).set_value(node);
        self.superblock.bump_index += 1;
        self.superblock.size += 1;
        i
    }

    /// Removes the node at index `i` from the allocator and adds the index to the free list
    /// When deleting nodes, you MUST clear all registers prior to calling `remove_node`
    fn remove_node(&mut self, i: u32) -> Option<&T> {
        if i == SENTINEL {
            return None;
        }
        if self.superblock.last_removed_node != SENTINEL {
            panic!("You have to clean up the last_removed_node by calling `start_pop_up` and `finish_pop_up` correctly before calling `remove_node`");
        }

        self.superblock.last_removed_node = i;
        self.superblock.size -= 1;
        Some(self.get(i).get_value())
    }
}
