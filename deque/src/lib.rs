use bytemuck::{Pod, Zeroable};
use node_allocator::{Node, NodeAllocator, ZeroCopy, SENTINEL};

// Register aliases
pub const PREV: u32 = 0;
pub const NEXT: u32 = 1;

#[derive(Copy, Clone)]
pub struct Deque<const MAX_SIZE: usize, T: Default + Copy + Clone + Pod + Zeroable> {
    pub sequence_number: u64,
    pub head: u32,
    pub tail: u32,
    allocator: NodeAllocator<MAX_SIZE, 2, T>,
}

unsafe impl<const MAX_SIZE: usize, T: Default + Copy + Clone + Pod + Zeroable> Zeroable
    for Deque<MAX_SIZE, T>
{
}
unsafe impl<const MAX_SIZE: usize, T: Default + Copy + Clone + Pod + Zeroable> Pod
    for Deque<MAX_SIZE, T>
{
}

impl<const MAX_SIZE: usize, T: Default + Copy + Clone + Pod + Zeroable> ZeroCopy
    for Deque<MAX_SIZE, T>
{
}

impl<const MAX_SIZE: usize, T: Default + Copy + Clone + Pod + Zeroable> Default
    for Deque<MAX_SIZE, T>
{
    fn default() -> Self {
        Deque {
            sequence_number: 0,
            head: SENTINEL,
            tail: SENTINEL,
            allocator: NodeAllocator::<MAX_SIZE, 2, T>::default(),
        }
    }
}

impl<const MAX_SIZE: usize, T: Default + Copy + Clone + Pod + Zeroable> Deque<MAX_SIZE, T> {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn front(&self) -> Option<&T> {
        if self.head == SENTINEL {
            return None;
        }
        Some(self.get(self.head).get_value())
    }

    pub fn back(&self) -> Option<&T> {
        if self.tail == SENTINEL {
            return None;
        }
        Some(self.allocator.get(self.tail).get_value())
    }

    #[inline(always)]
    fn get(&self, i: u32) -> &Node<2, T> {
        self.allocator.get(i)
    }

    pub fn push_back(&mut self, node: T) {
        let index = self.allocator.add_node(node);
        if self.head == SENTINEL {
            self.head = index;
        }
        if self.tail != SENTINEL {
            self.allocator.connect(index, self.tail, PREV, NEXT);
        }
        self.tail = index;
        self.sequence_number += 1;
    }

    pub fn push_front(&mut self, node: T) {
        let index = self.allocator.add_node(node);
        if self.tail == SENTINEL {
            self.tail = index;
        }
        if self.head != SENTINEL {
            self.allocator.connect(index, self.head, NEXT, PREV);
        }
        self.head = index;
        self.sequence_number += 1;
    }

    pub fn pop_front(&mut self) -> Option<&T> {
        if self.head == SENTINEL {
            return None;
        }
        let head_node = self.get(self.head);
        let new_head = head_node.get_register(NEXT as usize);
        let res = self.allocator.remove_node(self.head).unwrap();
        self.head = new_head;
        self.sequence_number += 1;
        Some(res)
    }

    pub fn pop_back(&mut self) -> Option<&T> {
        if self.tail == SENTINEL {
            return None;
        }
        let tail_node = self.get(self.tail);
        let new_tail = tail_node.get_register(PREV as usize);
        let res = self.allocator.remove_node(self.tail).unwrap();
        self.tail = new_tail;
        self.sequence_number += 1;
        Some(res)
    }

    pub fn remove(&mut self, i: usize) -> Option<T> {
        let (left, right, value) = {
            let node = self.get(i as u32);
            let value = *node.get_value();
            let left = node.get_register(PREV as usize);
            let right = node.get_register(NEXT as usize);
            (left, right, value)
        };
        self.allocator.clear_register(i as u32, PREV);
        self.allocator.clear_register(i as u32, NEXT);
        if left != SENTINEL && right != SENTINEL {
            self.allocator.connect(left, right, NEXT, PREV);
        }
        if i == self.head as usize {
            self.head = right;
            self.allocator.clear_register(right, PREV);
        }
        if i == self.tail as usize {
            self.tail = left;
            self.allocator.clear_register(left, NEXT);
        }
        self.allocator.remove_node(i as u32);
        self.sequence_number += 1;
        Some(value)
    }

    pub fn len(&self) -> usize {
        self.allocator.size as usize
    }

    pub fn iter(&self) -> DequeIterator<'_, T> {
        DequeIterator::<T> {
            nodes: &self.allocator.nodes,
            ptr: self.head,
        }
    }

    pub fn iter_mut(&mut self) -> DequeIteratorMut<'_, T> {
        DequeIteratorMut::<T> {
            nodes: &mut self.allocator.nodes,
            ptr: self.head,
        }
    }
}

pub struct DequeIterator<'a, T: Default + Copy + Clone + Pod + Zeroable> {
    nodes: &'a [Node<2, T>],
    ptr: u32,
}

impl<'a, T: Default + Copy + Clone + Pod + Zeroable> Iterator for DequeIterator<'a, T> {
    type Item = (usize, &'a T);

    fn next(&mut self) -> Option<Self::Item> {
        match self.ptr {
            SENTINEL => None,
            _ => {
                let ptr = self.ptr as usize;
                self.ptr = self.nodes[ptr as usize].get_register(NEXT as usize);
                Some((ptr, self.nodes[ptr as usize].get_value()))
            }
        }
    }
}

pub struct DequeIteratorMut<'a, T: Default + Copy + Clone + Pod + Zeroable> {
    nodes: &'a mut [Node<2, T>],
    ptr: u32,
}

impl<'a, T: Default + Copy + Clone + Pod + Zeroable> Iterator for DequeIteratorMut<'a, T> {
    type Item = (usize, &'a mut T);

    fn next(&mut self) -> Option<Self::Item> {
        match self.ptr {
            SENTINEL => None,
            _ => {
                let ptr = self.ptr as usize;
                self.ptr = self.nodes[ptr].get_register(NEXT as usize);
                Some((ptr, unsafe {
                    (*self.nodes.as_mut_ptr().add(ptr)).get_value_mut()
                }))
            }
        }
    }
}
