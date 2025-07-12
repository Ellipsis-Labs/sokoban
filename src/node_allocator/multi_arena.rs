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
    pub size: u32,

    /// The max size of the allocator.
    pub max_size: u32,

    /// Number of arenas in the superblock in use
    /// if `num_arenas` is less than the buffers we have, it means that the remaining buffers are not initialized
    /// only the last buffer can has less than arena_size space allocated
    pub num_active_arenas: u32,

    /// The size (number of nodes) of each arena ( 10MB / std::mem::size_of::<Node<T, NUM_REGISTERS>>() if it's a Solana Account )
    pub arena_size: u32,

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

    _padding: u64,
}

unsafe impl Zeroable for Superblock {}
unsafe impl Pod for Superblock {}
impl ZeroCopy for Superblock {}

impl Superblock {
    pub fn initialize(&mut self, num_active_arenas: usize, max_size: usize, arena_size: usize) {
        if self.size == 0
            && self.num_active_arenas == 0
            && self.bump_index == 0
            && self.free_list_head == 0
            && self.max_size == 0
            && self.arena_size == 0
        {
            self.max_size = max_size as u32;
            self.num_active_arenas = num_active_arenas as u32;
            self.arena_size = arena_size as u32;
            self.bump_index = 1;
            self.free_list_head = 1;
        } else {
            panic!("Cannot reinitialize NodeAllocator");
        }
    }
}

pub struct MultiArenaNodeAllocator<
    'a,
    T: Default + Copy + Clone + Pod + Zeroable,
    const NUM_REGISTERS: usize,
> {
    pub superblock: &'a mut Superblock,
    pub arenas: Vec<&'a mut [Node<T, NUM_REGISTERS>]>,
}

impl<'a, T: Default + Copy + Clone + Pod + Zeroable, const NUM_REGISTERS: usize>
    MultiArenaNodeAllocator<'a, T, NUM_REGISTERS>
{
    fn new(
        superblock: &'a mut Superblock,
        arenas: Vec<&'a mut [Node<T, NUM_REGISTERS>]>,
    ) -> &'a mut Self {
        Box::leak(Box::new(Self { superblock, arenas }))
    }

    #[inline(always)]
    fn index_conv(&self, i: u32) -> (usize, usize) {
        let page_no = i - 1;
        // Can be optimized to use bit math if the arena size is a power of two
        // but doesn't really matter in Solana world
        let block_no = page_no / self.superblock.arena_size as u32;
        let node_no = page_no % self.superblock.arena_size as u32;
        (block_no as usize, node_no as usize)
    }

    pub fn from_buffers(
        superblock_buffer: &'a mut [u8],
        arena_buffers: &'a mut [&'a mut [u8]],
    ) -> &'a mut Self {
        let superblock = Superblock::load_mut_bytes(superblock_buffer)
            .expect("Failed to load Superblock from buffer");

        if superblock.num_active_arenas == 0 || superblock.max_size == 0 {
            panic!("Superblock must be pre-initialized and has non-empty arenas and max size")
        }

        let mut arenas = Vec::with_capacity(arena_buffers.len());

        for arena_buffer in arena_buffers.iter_mut() {
            let nodes = bytemuck::cast_slice_mut::<u8, Node<T, NUM_REGISTERS>>(arena_buffer);
            arenas.push(nodes);
        }
        for arena in arenas.iter().rev().skip(1) {
            assert_eq!(
                arena.len(),
                superblock.arena_size as usize,
                "Only the last arena can be smaller than the arena size"
            );
        }

        // active arenas: consecutive non-empty arenas, all arena after the last active arena must be empty
        // only the last arena can be smaller than the arena size, all other arenas must be full

        // discover if the number of active arena has been increased
        // Find the index of the last non-empty arena (active arenas are consecutive from the start)
        let mut num_active_arenas = 0;
        for (i, arena) in arenas.iter().enumerate() {
            if !arena.is_empty() {
                num_active_arenas = i + 1;
            }
        }

        assert!(
            arenas[num_active_arenas - 1].len() <= superblock.arena_size as usize,
            "Arena at index {} has larger size than the arena size ({})",
            num_active_arenas - 1,
            superblock.arena_size
        );

        // validate arenas
        // 1. all active arenas must be non-empty
        // 2. all arenas besides the last one must be full

        // All arenas after the last active arena must be empty
        for (i, arena) in arenas.iter().enumerate().skip(num_active_arenas) {
            if !arena.is_empty() {
                panic!(
                    "Arena at index {} is not empty, but it is after the last active arena ({}).",
                    i,
                    num_active_arenas - 1
                );
            }
        }

        // Only the last arena can be smaller than the arena size, all other arenas must be full (therefore must be non-empty)
        for (i, arena) in arenas
            .iter()
            .enumerate()
            .take(num_active_arenas.saturating_sub(1))
        {
            if arena.len() != superblock.arena_size as usize {
                panic!(
                    "Arena at index {} has a size of {} but expected {}. Only the last arena is allowed to be smaller than the standard arena size.",
                    i,
                    arena.len(),
                    superblock.arena_size
                );
            }
        }

        // correctly set the number of arenas, if it has been initialized
        superblock.num_active_arenas = num_active_arenas as u32;

        // correctly set the max size
        let current_max_size = arenas.iter().map(|arena| arena.len() as u32).sum::<u32>();
        if current_max_size < superblock.max_size {
            panic!(
                "Current max size ({}) is less than the max size of the allocator ({})",
                current_max_size, superblock.max_size
            );
        } else {
            superblock.max_size = current_max_size;
        }

        Self::new(superblock, arenas)
    }

    // the free list must be empty when calling this
    // it returns the space freed if the allocator can be resized down, otherwise it returns None
    // FIXME: not sure if the interface is correct, check it out when implementing the utilities around smart contract
    pub fn resize_down(&mut self) -> Option<u32> {
        if self.superblock.free_list_head != self.superblock.bump_index {
            return None;
        }
        if self.superblock.bump_index == self.superblock.size as u32 + 1 {
            return None;
        }
        let mut nodes_freed = self.superblock.size + 1 - self.superblock.bump_index;
        let space_freed = nodes_freed * std::mem::size_of::<Node<T, NUM_REGISTERS>>() as u32;
        self.superblock.bump_index = self.superblock.size as u32 + 1;
        self.superblock.free_list_head = self.superblock.bump_index;

        // pop off unused arenas by decreasing `num_active_arenas`
        let mut last_active_arena = self.superblock.num_active_arenas as usize - 1;
        while nodes_freed > self.arenas[last_active_arena].len() as u32 {
            nodes_freed -= self.arenas[last_active_arena].len() as u32;
            last_active_arena -= 1;
        }
        self.superblock.num_active_arenas = last_active_arena as u32 + 1;
        Some(space_freed)
    }

    // Haven't figured out what the resize_up interface should be
    // right now we can just reload the whole data structure from buffers to perform a resize up
}

impl<'a, T: Default + Copy + Clone + Pod + Zeroable, const NUM_REGISTERS: usize>
    NodeAllocator<T, NUM_REGISTERS> for MultiArenaNodeAllocator<'a, T, NUM_REGISTERS>
{
    fn assert_proper_alignment(&self) {
        let reg_size = size_of::<u32>() * NUM_REGISTERS;
        let self_ptr = std::slice::from_ref(self).as_ptr() as usize;
        let self_align = align_of::<Self>();
        assert!(
            self_ptr % self_align as usize == 0,
            "NodeAllocator alignment mismatch, address is {} which is not a multiple of the struct alignment ({})",
            self_ptr,
            self_align,
        );
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
    }

    fn size(&self) -> usize {
        self.superblock.size as usize
    }

    fn capacity(&self) -> usize {
        self.superblock.max_size as usize
    }

    fn initialize(&mut self) {
        assert!(NUM_REGISTERS >= 1);
        self.assert_proper_alignment();
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
        let i = self.superblock.free_list_head;
        if self.superblock.free_list_head == self.superblock.bump_index {
            if self.superblock.bump_index == self.superblock.max_size as u32 + 1 {
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
