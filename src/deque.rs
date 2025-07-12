use crate::{
    container::Container,
    node_allocator::{
        MultiArenaNodeAllocator, NodeAllocator, SimpleNodeAllocator, ZeroCopy, SENTINEL,
    },
};
use bytemuck::{Pod, Zeroable};

// Register aliases
pub const PREV: u32 = 0;
pub const NEXT: u32 = 1;

#[repr(C)]
#[derive(Copy, Clone)]
pub struct DequeHeader {
    pub sequence_number: u64,
    pub head: u32,
    pub tail: u32,
}

unsafe impl Zeroable for DequeHeader {}
unsafe impl Pod for DequeHeader {}
impl ZeroCopy for DequeHeader {}
impl Default for DequeHeader {
    fn default() -> Self {
        Self {
            sequence_number: 0,
            head: SENTINEL,
            tail: SENTINEL,
        }
    }
}

pub type Deque<'a, T, Allocator> = Container<'a, DequeHeader, T, Allocator, 2>;
pub type StaticDeque<'a, T, const MAX_SIZE: usize> =
    Deque<'a, T, SimpleNodeAllocator<T, MAX_SIZE, 2>>;
pub type DynamicDeque<'a, T> = Deque<'a, T, MultiArenaNodeAllocator<'a, T, 2>>;

impl<'a, T: Default + Copy + Clone + Pod + Zeroable, Allocator: NodeAllocator<T, 2>>
    Deque<'a, T, Allocator>
{
    pub fn initialize(&mut self) {
        self.allocator.initialize();
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

    pub fn pop_front(&mut self) -> Option<T> {
        if self.head == SENTINEL {
            return None;
        }
        let head = self.head;
        self._remove(head)
    }

    pub fn pop_back(&mut self) -> Option<T> {
        if self.tail == SENTINEL {
            return None;
        }
        let tail = self.tail;
        self._remove(tail)
    }

    fn _remove(&mut self, i: u32) -> Option<T> {
        let (left, right, value) = {
            let value = *self.get_node(i);
            let left = self.get_prev(i);
            let right = self.get_next(i);
            (left, right, value)
        };
        self.allocator.clear_register(i as u32, PREV);
        self.allocator.clear_register(i as u32, NEXT);
        if left != SENTINEL && right != SENTINEL {
            self.allocator.connect(left, right, NEXT, PREV);
        }
        if i == self.head {
            self.head = right;
            self.allocator.clear_register(right, PREV);
        }
        if i == self.tail {
            self.tail = left;
            self.allocator.clear_register(left, NEXT);
        }
        self.allocator.remove_node(i as u32);
        self.sequence_number += 1;
        Some(value)
    }

    pub fn len(&self) -> usize {
        self.allocator.size() as usize
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn iter(&self) -> DequeIterator<'_, T, Allocator> {
        DequeIterator::<T, Allocator> {
            deque: self,
            fwd_ptr: self.head,
            rev_ptr: self.tail,
            terminated: false,
        }
    }

    pub fn iter_mut<'deque>(&'deque mut self) -> DequeIteratorMut<'deque, 'a, T, Allocator>
    where
        'a: 'deque,
    {
        let head = self.head;
        let tail = self.tail;
        DequeIteratorMut::<'deque, 'a, T, Allocator> {
            deque: self,
            fwd_ptr: head,
            rev_ptr: tail,
            terminated: false,
        }
    }
}

pub struct DequeIterator<
    'a,
    T: Default + Copy + Clone + Pod + Zeroable,
    Allocator: NodeAllocator<T, 2>,
> {
    deque: &'a Deque<'a, T, Allocator>,
    fwd_ptr: u32,
    rev_ptr: u32,
    terminated: bool,
}

impl<'a, T: Default + Copy + Clone + Pod + Zeroable, Allocator: NodeAllocator<T, 2>> Iterator
    for DequeIterator<'a, T, Allocator>
{
    type Item = (usize, &'a T);

    fn next(&mut self) -> Option<Self::Item> {
        if self.terminated {
            return None;
        }
        match self.fwd_ptr {
            SENTINEL => None,
            _ => {
                let ptr = self.fwd_ptr;
                if ptr == self.rev_ptr {
                    self.terminated = true;
                }
                self.fwd_ptr = self.deque.get_next(ptr);
                Some((ptr as usize, self.deque.get_node(ptr)))
            }
        }
    }
}

impl<'a, T: Default + Copy + Clone + Pod + Zeroable, Allocator: NodeAllocator<T, 2>>
    DoubleEndedIterator for DequeIterator<'a, T, Allocator>
{
    fn next_back(&mut self) -> Option<Self::Item> {
        if self.terminated {
            return None;
        }
        match self.rev_ptr {
            SENTINEL => None,
            _ => {
                let ptr = self.rev_ptr;
                if ptr == self.fwd_ptr {
                    self.terminated = true;
                }
                self.rev_ptr = self.deque.get_prev(ptr);
                Some((ptr as usize, self.deque.get_node(ptr)))
            }
        }
    }
}

pub struct DequeIteratorMut<
    'deque,
    'a,
    T: Default + Copy + Clone + Pod + Zeroable,
    Allocator: NodeAllocator<T, 2>,
> where
    'a: 'deque,
{
    deque: &'deque mut Deque<'a, T, Allocator>,
    fwd_ptr: u32,
    rev_ptr: u32,
    terminated: bool,
}

impl<'deque, 'a, T: Default + Copy + Clone + Pod + Zeroable, Allocator: NodeAllocator<T, 2>>
    Iterator for DequeIteratorMut<'deque, 'a, T, Allocator>
where
    'a: 'deque,
{
    type Item = (usize, &'a mut T);

    fn next(&mut self) -> Option<Self::Item> {
        if self.terminated {
            return None;
        }
        match self.fwd_ptr {
            SENTINEL => None,
            _ => {
                let ptr = self.fwd_ptr;
                if ptr == self.rev_ptr {
                    self.terminated = true;
                }
                self.fwd_ptr = self.deque.get_next(ptr);

                // SAFETY: This is required to extend the lifetime of the mutable reference
                // to 'a, but Rust's borrow checker cannot prove this is safe. The iterator
                // guarantees only one mutable reference to each node at a time, and the
                // iterator itself is unique, so this is sound as long as the iterator is
                // not misused (e.g., aliased or cloned).
                unsafe {
                    let item: &mut T =
                        &mut *(&mut *self.deque.allocator.get_mut(ptr).get_value_mut() as *mut _);
                    Some((ptr as usize, item))
                }
            }
        }
    }
}

impl<'deque, 'a, T: Default + Copy + Clone + Pod + Zeroable, Allocator: NodeAllocator<T, 2>>
    DoubleEndedIterator for DequeIteratorMut<'deque, 'a, T, Allocator>
where
    'a: 'deque,
{
    fn next_back(&mut self) -> Option<Self::Item> {
        if self.terminated {
            return None;
        }
        match self.rev_ptr {
            SENTINEL => None,
            _ => {
                let ptr = self.rev_ptr;
                if ptr == self.fwd_ptr {
                    self.terminated = true;
                }
                self.rev_ptr = self.deque.get_prev(ptr);

                // SAFETY: This is required to extend the lifetime of the mutable reference
                // to 'a, but Rust's borrow checker cannot prove this is safe. The iterator
                // guarantees only one mutable reference to each node at a time, and the
                // iterator itself is unique, so this is sound as long as the iterator is
                // not misused (e.g., aliased or cloned).
                unsafe {
                    let item: &mut T =
                        &mut *(&mut *self.deque.allocator.get_mut(ptr).get_value_mut() as *mut _);
                    Some((ptr as usize, item))
                }
            }
        }
    }
}

#[test]
/// This test covers the primary use cases of the deque
fn test_deque() {
    use rand::thread_rng;
    use rand::Rng;
    use std::collections::VecDeque;
    let mut rng = thread_rng();
    type Q<'a> = StaticDeque<'a, u64, 1024>;
    let mut buf = vec![0u8; Q::size_of_buffer()];
    let mut v = VecDeque::new();
    let mut q = Q::load_from_buffer(buf.as_mut_slice());
    q.initialize();
    (0..128).for_each(|_| {
        let t = rng.gen::<u64>();
        q.push_back(t);
        v.push_back(t);
    });
    (0..128).for_each(|_| {
        let t = rng.gen::<u64>();
        q.push_front(t);
        v.push_front(t);
    });
    for ((_, i), j) in q.iter().zip(v.iter()) {
        assert_eq!(i, j);
    }
    for ((_, i), j) in q.iter().rev().zip(v.iter().rev()) {
        assert_eq!(i, j);
    }

    {
        let mut q_iter = q.iter();
        let mut v_iter = v.iter();
        let breakpoint = rng.gen_range(1, 255);
        for _ in 0..breakpoint {
            assert_eq!(q_iter.next().map(|x| x.1), v_iter.next());
        }
        for _ in breakpoint..256 {
            assert_eq!(q_iter.next_back().map(|x| x.1), v_iter.next_back());
        }

        assert!(q_iter.next().is_none());
        assert!(q_iter.next_back().is_none());
        assert!(v_iter.next().is_none());
        assert!(v_iter.next_back().is_none());
        // Do it again for good measure
        assert!(q_iter.next().is_none());
        assert!(q_iter.next_back().is_none());
        assert!(v_iter.next().is_none());
        assert!(v_iter.next_back().is_none());
    }

    {
        let mut q_iter_mut = q.iter_mut();
        let mut v_iter_mut = v.iter_mut();
        let breakpoint = rng.gen_range(1, 255);
        for _ in 0..breakpoint {
            assert_eq!(q_iter_mut.next().map(|x| x.1), v_iter_mut.next());
        }
        for _ in breakpoint..256 {
            assert_eq!(q_iter_mut.next_back().map(|x| x.1), v_iter_mut.next_back());
        }

        assert!(q_iter_mut.next().is_none());
        assert!(q_iter_mut.next_back().is_none());
        assert!(v_iter_mut.next().is_none());
        assert!(v_iter_mut.next_back().is_none());
        // Do it again for good measure
        assert!(q_iter_mut.next().is_none());
        assert!(q_iter_mut.next_back().is_none());
        assert!(v_iter_mut.next().is_none());
        assert!(v_iter_mut.next_back().is_none());
    }

    (0..256).for_each(|_| {
        assert_eq!(q.pop_back(), v.pop_back());
    });
    assert!(q.is_empty() && v.is_empty());
    (0..128).for_each(|_| {
        let t = rng.gen::<u64>();
        q.push_back(t);
        v.push_back(t);
    });
    (0..128).for_each(|_| {
        let t = rng.gen::<u64>();
        q.push_front(t);
        v.push_front(t);
    });
    for ((_, i), j) in q.iter().zip(v.iter()) {
        assert_eq!(i, j);
    }
    for ((_, i), j) in q.iter().rev().zip(v.iter().rev()) {
        assert_eq!(i, j);
    }
    (0..256).for_each(|_| {
        assert_eq!(q.pop_front(), v.pop_front());
    });
    assert!(q.is_empty() && v.is_empty());
}
