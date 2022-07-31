use bytemuck::{Pod, Zeroable};

pub const SENTINEL: u32 = 0;

#[repr(C)]
#[derive(Copy, Clone)]
pub struct Node<const NUM_REGISTERS: usize, T: Copy + Clone + Pod + Zeroable + Default> {
    /// Arbitrary registers (generally used for pointers)
    /// Note: Register 0 is ALWAYS used for the free list
    registers: [u32; NUM_REGISTERS],
    value: T,
}

impl<const NUM_REGISTERS: usize, T: Copy + Clone + Pod + Zeroable + Default> Default
    for Node<NUM_REGISTERS, T>
{
    fn default() -> Self {
        assert!(NUM_REGISTERS >= 1);
        Self {
            registers: [SENTINEL; NUM_REGISTERS],
            value: T::default(),
        }
    }
}

impl<const NUM_REGISTERS: usize, T: Copy + Clone + Pod + Zeroable + Default>
    Node<NUM_REGISTERS, T>
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

#[derive(Copy, Clone)]
pub struct NodeAllocator<
    const MAX_SIZE: usize,
    const NUM_REGISTERS: usize,
    T: Default + Copy + Clone + Pod + Zeroable,
> {
    /// Size of the allocator
    pub size: u64,
    /// Furthest index of the allocator
    bump_index: u32,
    /// Buffer index of the first element in the free list
    free_list_head: u32,
    pub nodes: [Node<NUM_REGISTERS, T>; MAX_SIZE],
}

unsafe impl<
        const MAX_SIZE: usize,
        const NUM_REGISTERS: usize,
        T: Default + Copy + Clone + Pod + Zeroable,
    > Zeroable for NodeAllocator<MAX_SIZE, NUM_REGISTERS, T>
{
}
unsafe impl<
        const MAX_SIZE: usize,
        const NUM_REGISTERS: usize,
        T: Default + Copy + Clone + Pod + Zeroable,
    > Pod for NodeAllocator<MAX_SIZE, NUM_REGISTERS, T>
{
}

impl<
        const MAX_SIZE: usize,
        const NUM_REGISTERS: usize,
        T: Default + Copy + Clone + Pod + Zeroable,
    > Default for NodeAllocator<MAX_SIZE, NUM_REGISTERS, T>
{
    fn default() -> Self {
        NodeAllocator {
            size: 0,
            bump_index: 1,
            free_list_head: 1,
            nodes: [Node::<NUM_REGISTERS, T>::default(); MAX_SIZE],
        }
    }
}

impl<
        const MAX_SIZE: usize,
        const NUM_REGISTERS: usize,
        T: Default + Copy + Clone + Pod + Zeroable,
    > NodeAllocator<MAX_SIZE, NUM_REGISTERS, T>
{
    pub fn new() -> Self {
        Self::default()
    }

    pub fn init_default(&mut self) {
        assert!(NUM_REGISTERS >= 1);
        if self.size == 0 {
            self.bump_index = 1;
            self.free_list_head = 1;
        }
    }

    #[inline(always)]
    pub fn get(&self, i: u32) -> &Node<NUM_REGISTERS, T> {
        &self.nodes[i as usize]
    }

    #[inline(always)]
    pub fn get_mut(&mut self, i: u32) -> &mut Node<NUM_REGISTERS, T> {
        &mut self.nodes[i as usize]
    }

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
        self.clear_register(i, r_i);
        self.clear_register(j, r_j);
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
        self.get_mut(i).set_register(r_i as usize, value);
    }

    #[inline(always)]
    pub fn get_register(&self, i: u32, r_i: u32) -> u32 {
        self.get(i).get_register(r_i as usize)
    }
}
