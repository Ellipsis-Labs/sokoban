use crate::node_allocator::{NodeAllocator, ZeroCopy, SENTINEL};
use bytemuck::{Pod, Zeroable};

// Register aliases
pub const PREV: u32 = 0;
pub const NEXT: u32 = 1;

#[derive(Copy, Clone)]
pub struct Deque<T: Default + Copy + Clone + Pod + Zeroable, const MAX_SIZE: usize> {
    pub sequence_number: u64,
    pub head: u32,
    pub tail: u32,
    allocator: NodeAllocator<T, MAX_SIZE, 2>,
}

unsafe impl<T: Default + Copy + Clone + Pod + Zeroable, const MAX_SIZE: usize> Zeroable
    for Deque<T, MAX_SIZE>
{
}
unsafe impl<T: Default + Copy + Clone + Pod + Zeroable, const MAX_SIZE: usize> Pod
    for Deque<T, MAX_SIZE>
{
}

impl<T: Default + Copy + Clone + Pod + Zeroable, const MAX_SIZE: usize> ZeroCopy
    for Deque<T, MAX_SIZE>
{
}

impl<T: Default + Copy + Clone + Pod + Zeroable, const MAX_SIZE: usize> Default
    for Deque<T, MAX_SIZE>
{
    fn default() -> Self {
        Deque {
            sequence_number: 0,
            head: SENTINEL,
            tail: SENTINEL,
            allocator: NodeAllocator::<T, MAX_SIZE, 2>::default(),
        }
    }
}

impl<T: Default + Copy + Clone + Pod + Zeroable, const MAX_SIZE: usize> Deque<T, MAX_SIZE> {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn front(&self) -> Option<&T> {
        if self.head == SENTINEL {
            return None;
        }
        Some(self.get_node(self.head))
    }

    pub fn back(&self) -> Option<&T> {
        if self.tail == SENTINEL {
            return None;
        }
        Some(self.allocator.get(self.tail).get_value())
    }

    pub fn get_next(&self, index: u32) -> u32 {
        self.allocator.get_register(index, NEXT)
    }

    pub fn get_prev(&self, index: u32) -> u32 {
        self.allocator.get_register(index, PREV)
    }

    #[inline(always)]
    fn get_node(&self, i: u32) -> &T {
        self.allocator.get(i).get_value()
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
        let new_head = self.get_next(self.head);
        let res = self.allocator.remove_node(self.head).unwrap();
        self.head = new_head;
        self.sequence_number += 1;
        Some(res)
    }

    pub fn pop_back(&mut self) -> Option<&T> {
        if self.tail == SENTINEL {
            return None;
        }
        let new_tail = self.get_prev(self.tail);
        let res = self.allocator.remove_node(self.tail).unwrap();
        self.tail = new_tail;
        self.sequence_number += 1;
        Some(res)
    }

    pub fn remove(&mut self, i: usize) -> Option<T> {
        let (left, right, value) = {
            let value = *self.get_node(i as u32);
            let left = self.get_prev(i as u32);
            let right = self.get_next(i as u32);
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

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn iter(&self) -> DequeIterator<'_, T, MAX_SIZE> {
        DequeIterator::<T, MAX_SIZE> {
            deque: self,
            ptr: self.head,
        }
    }

    pub fn iter_mut(&mut self) -> DequeIteratorMut<'_, T, MAX_SIZE> {
        let head = self.head;
        DequeIteratorMut::<T, MAX_SIZE> {
            deque: self,
            ptr: head,
        }
    }
}

pub struct DequeIterator<'a, T: Default + Copy + Clone + Pod + Zeroable, const MAX_SIZE: usize> {
    deque: &'a Deque<T, MAX_SIZE>,
    ptr: u32,
}

impl<'a, T: Default + Copy + Clone + Pod + Zeroable, const MAX_SIZE: usize> Iterator
    for DequeIterator<'a, T, MAX_SIZE>
{
    type Item = (usize, &'a T);

    fn next(&mut self) -> Option<Self::Item> {
        match self.ptr {
            SENTINEL => None,
            _ => {
                let ptr = self.ptr;
                self.ptr = self.deque.get_next(ptr);
                Some((ptr as usize, self.deque.get_node(ptr)))
            }
        }
    }
}

pub struct DequeIteratorMut<'a, T: Default + Copy + Clone + Pod + Zeroable, const MAX_SIZE: usize> {
    deque: &'a mut Deque<T, MAX_SIZE>,
    ptr: u32,
}

impl<'a, T: Default + Copy + Clone + Pod + Zeroable, const MAX_SIZE: usize> Iterator
    for DequeIteratorMut<'a, T, MAX_SIZE>
{
    type Item = (usize, &'a mut T);

    fn next(&mut self) -> Option<Self::Item> {
        match self.ptr {
            SENTINEL => None,
            _ => {
                let ptr = self.ptr;
                self.ptr = self.deque.get_next(ptr);
                Some((ptr as usize, unsafe {
                    (*self.deque.allocator.nodes.as_mut_ptr().add(ptr as usize)).get_value_mut()
                }))
            }
        }
    }
}
