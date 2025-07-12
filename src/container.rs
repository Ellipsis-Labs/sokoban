use std::{
    marker::PhantomData,
    ops::{Deref, DerefMut},
};

use bytemuck::{Pod, Zeroable};

use crate::{
    node_allocator::{MultiArenaNodeAllocator, SimpleNodeAllocator, Superblock},
    NodeAllocator, ZeroCopy,
};

pub trait AssertProperAlignment {
    fn assert_proper_alignment() {}
}

pub struct Container<
    'a,
    HeaderType: Pod + Zeroable + ZeroCopy + AssertProperAlignment,
    NodeType: Pod + Zeroable + Copy + Default,
    Allocator: NodeAllocator<NodeType, NUM_REGISTERS>,
    const NUM_REGISTERS: usize,
> {
    pub header: &'a mut HeaderType,
    pub allocator: &'a mut Allocator,
    pub _phantom: PhantomData<NodeType>,
}

impl<
        'a,
        HeaderType: Pod + Zeroable + ZeroCopy + AssertProperAlignment,
        NodeType: Pod + Zeroable + Copy + Default,
        const MAX_SIZE: usize,
        const NUM_REGISTERS: usize,
    >
    Container<
        'a,
        HeaderType,
        NodeType,
        SimpleNodeAllocator<NodeType, MAX_SIZE, NUM_REGISTERS>,
        NUM_REGISTERS,
    >
{
    pub fn load_from_buffer(buf: &'a mut [u8]) -> Self {
        let (header_slice, allocator_slice) = buf.split_at_mut(std::mem::size_of::<HeaderType>());
        let header = HeaderType::load_mut_bytes(header_slice).expect("Failed to load header");
        let allocator =
            SimpleNodeAllocator::load_mut_bytes(allocator_slice).expect("Failed to load allocator");

        allocator.assert_proper_alignment();

        Self {
            header,
            allocator,
            _phantom: PhantomData,
        }
    }

    pub fn new_from_buffer(buf: &'a mut [u8]) -> Self {
        let mut this = Self::load_from_buffer(buf);
        this.initialize();
        this
    }

    pub fn size_of_buffer() -> usize {
        std::mem::size_of::<HeaderType>()
            + std::mem::size_of::<SimpleNodeAllocator<NodeType, MAX_SIZE, NUM_REGISTERS>>()
    }
}

impl<
        'a,
        HeaderType: Pod + Zeroable + ZeroCopy + AssertProperAlignment,
        NodeType: Pod + Zeroable + Copy + Default,
        const NUM_REGISTERS: usize,
    >
    Container<
        'a,
        HeaderType,
        NodeType,
        MultiArenaNodeAllocator<'a, NodeType, NUM_REGISTERS>,
        NUM_REGISTERS,
    >
{
    pub fn load_from_buffers(buf: &'a mut [u8], arena_bufs: &'a mut [&'a mut [u8]]) -> Self {
        let (superblock_buf, header_buf) = buf.split_at_mut(std::mem::size_of::<Superblock>());

        let allocator = MultiArenaNodeAllocator::from_buffers(superblock_buf, arena_bufs);
        let header = HeaderType::load_mut_bytes(header_buf).expect("Failed to load header");

        allocator.assert_proper_alignment();

        Self {
            header,
            allocator,
            _phantom: PhantomData,
        }
    }

    pub fn new_from_buffers(
        buf: &'a mut [u8],
        arena_bufs: &'a mut [&'a mut [u8]],
        num_arenas: usize,
        max_size: usize,
        arena_size: usize,
    ) -> Self {
        let (superblock_buf, header_buf) = buf.split_at_mut(std::mem::size_of::<Superblock>());

        // initialize the superblock first
        {
            let superblock =
                Superblock::load_mut_bytes(superblock_buf).expect("Failed to load superblock");
            superblock.initialize(num_arenas, max_size, arena_size);
        }

        let allocator = MultiArenaNodeAllocator::from_buffers(superblock_buf, arena_bufs);
        let header = HeaderType::load_mut_bytes(header_buf).expect("Failed to load header");

        allocator.assert_proper_alignment();
        allocator.initialize();

        Self {
            header,
            allocator,
            _phantom: PhantomData,
        }
    }

    pub fn size_of_header() -> usize {
        std::mem::size_of::<Superblock>() + std::mem::size_of::<HeaderType>()
    }
}

impl<
        'a,
        HeaderType: Pod + Zeroable + ZeroCopy + AssertProperAlignment,
        NodeType: Pod + Zeroable + Copy + Default,
        Allocator: NodeAllocator<NodeType, NUM_REGISTERS>,
        const NUM_REGISTERS: usize,
    > Deref for Container<'a, HeaderType, NodeType, Allocator, NUM_REGISTERS>
{
    type Target = HeaderType;

    fn deref(&self) -> &Self::Target {
        self.header
    }
}

impl<
        'a,
        HeaderType: Pod + Zeroable + ZeroCopy + AssertProperAlignment,
        NodeType: Pod + Zeroable + Copy + Default,
        Allocator: NodeAllocator<NodeType, NUM_REGISTERS>,
        const NUM_REGISTERS: usize,
    > DerefMut for Container<'a, HeaderType, NodeType, Allocator, NUM_REGISTERS>
{
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.header
    }
}

impl<
        'a,
        HeaderType: Pod + Zeroable + ZeroCopy + AssertProperAlignment,
        NodeType: Pod + Zeroable + Copy + Default,
        Allocator: NodeAllocator<NodeType, NUM_REGISTERS>,
        const NUM_REGISTERS: usize,
    > Container<'a, HeaderType, NodeType, Allocator, NUM_REGISTERS>
{
    pub fn initialize(&mut self) {
        HeaderType::assert_proper_alignment();
        self.allocator.assert_proper_alignment();
        self.allocator.initialize();
    }
}
