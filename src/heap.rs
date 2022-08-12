/*
General implementation of a heap
*/
use bytemuck::{Pod, Zeroable};
use std::cmp::PartialOrd;

#[derive(Debug, Default, Clone, Copy)]
pub struct Node<T> {
    pub v: T,
}

#[derive(Debug, Clone)]
pub struct Heap<T: PartialOrd + Copy + Clone + Default + Pod + Zeroable, const MAX_SIZE: usize> {
    pub size: u64,
    pub nodes: [Node<T>; MAX_SIZE],
}

impl<T: PartialOrd + Copy + Clone + Default + Pod + Zeroable, const MAX_SIZE: usize> Default
    for Heap<T, MAX_SIZE>
{
    fn default() -> Self {
        Heap {
            size: 0,
            nodes: [Node::default(); MAX_SIZE],
        }
    }
}

impl<T: PartialOrd + Copy + Clone + Default + Pod + Zeroable, const MAX_SIZE: usize> Copy
    for Heap<T, MAX_SIZE>
{
}

unsafe impl<T: PartialOrd + Copy + Clone + Default + Pod + Zeroable, const MAX_SIZE: usize> Pod
    for Heap<T, MAX_SIZE>
{
}

unsafe impl<T: PartialOrd + Copy + Clone + Default + Pod + Zeroable, const MAX_SIZE: usize> Zeroable
    for Heap<T, MAX_SIZE>
{
}

impl<T: PartialOrd + Copy + Clone + Default + Pod + Zeroable, const MAX_SIZE: usize>
    Heap<T, MAX_SIZE>
{
    fn swap_node(arr: &mut [Node<T>; MAX_SIZE], parent_idx: usize, added_idx: usize) {
        let temp = arr[parent_idx];
        arr[parent_idx] = arr[added_idx];
        arr[added_idx] = temp;
    }

    pub fn _is_empty(&self) -> bool {
        return self.size == 0;
    }

    pub fn _size(&self) -> u64 {
        return self.size;
    }

    pub fn _peek(&self) -> T {
        return self.nodes[0].v;
    }

    fn _heapifyup(&mut self, index: usize) {
        if self.size == 1 {
            return;
        }
        if index == 0 {
            return;
        }
        let index: usize = index;
        let parent_index = (index - 1) / 2;

        if self.nodes[index].v > self.nodes[parent_index].v {
            Self::swap_node(&mut self.nodes, index, parent_index);
            self._heapifyup(parent_index)
        } else {
            return;
        }
    }

    fn _heapifydown(&mut self, rootidx: usize) {
        let rootidx = rootidx;
        let left_childidx = (2 * rootidx) + 1;
        let right_childidx = (2 * rootidx) + 2;

        if right_childidx <= self.size as usize {
            if self.nodes[left_childidx].v > self.nodes[right_childidx].v {
                if self.nodes[left_childidx].v > self.nodes[rootidx].v {
                    Self::swap_node(&mut self.nodes, rootidx, left_childidx);
                    self._heapifydown(left_childidx)
                }
            } else if self.nodes[right_childidx].v > self.nodes[left_childidx].v {
                if self.nodes[right_childidx].v > self.nodes[rootidx].v {
                    Self::swap_node(&mut self.nodes, rootidx, right_childidx);
                    self._heapifydown(right_childidx)
                }
            }
        } else if left_childidx <= self.size as usize {
            // right doesn't exist, no need to check right
            if self.nodes[left_childidx].v > self.nodes[rootidx].v {
                Self::swap_node(&mut self.nodes, rootidx, left_childidx);
                self._heapifydown(left_childidx)
            }
        }
    }

    pub fn _add(&mut self, value: T) {
        let node = Node::<T> { v: value };
        self.nodes[self.size as usize] = node;
        self._heapifyup(self.size as usize);
        self.size += 1;
    }

    pub fn _pop(&mut self) {
        let lastidx = (self.size - 1) as usize;
        Self::swap_node(&mut self.nodes, 0, lastidx);
        self.nodes[(self.size - 1) as usize] = Node::default();
        self.size -= 1;
        self._heapifydown(0);
    }
}

trait Min<T: PartialOrd + Copy + Clone + Default + Pod + Zeroable> {
    fn add_min(&mut self, value: T);
    fn pop_min(&mut self);
    fn heapify_up_min(&mut self, index: usize);
    fn heapify_down_min(&mut self, rootidx: usize);
}
/*
impl of functions for a min heap
*/
impl<T: PartialOrd + Copy + Clone + Default + Pod + Zeroable, const MAX_SIZE: usize> Min<T>
    for Heap<T, MAX_SIZE>
{
    fn add_min(&mut self, value: T) {
        let node = Node::<T> { v: value };
        self.nodes[self.size as usize] = node;
        self.heapify_up_min(self.size as usize);
        self.size += 1;
    }
    fn pop_min(&mut self) {
        let lastidx = (self.size - 1) as usize;
        Self::swap_node(&mut self.nodes, 0, lastidx);
        self.nodes[(self.size - 1) as usize] = Node::default();
        self.size -= 1;
        self.heapify_down_min(0);
    }
    fn heapify_up_min(&mut self, index: usize) {
        if self.size == 1 {
            return;
        }
        if index == 0 {
            return;
        }
        let index: usize = index;
        let parent_index = (index - 1) / 2;

        if self.nodes[index].v < self.nodes[parent_index].v {
            Self::swap_node(&mut self.nodes, index, parent_index);
            self.heapify_up_min(parent_index)
        } else {
            return;
        }
    }
    fn heapify_down_min(&mut self, rootidx: usize) {
        let rootidx = rootidx;
        let left_childidx = (2 * rootidx) + 1;
        let right_childidx = (2 * rootidx) + 2;

        if right_childidx <= self.size as usize {
            if self.nodes[left_childidx].v < self.nodes[right_childidx].v {
                if self.nodes[left_childidx].v < self.nodes[rootidx].v {
                    Self::swap_node(&mut self.nodes, rootidx, left_childidx);
                    self.heapify_down_min(left_childidx)
                }
            } else if self.nodes[right_childidx].v < self.nodes[left_childidx].v {
                if self.nodes[right_childidx].v < self.nodes[rootidx].v {
                    Self::swap_node(&mut self.nodes, rootidx, right_childidx);
                    self.heapify_down_min(right_childidx)
                }
            }
        } else if left_childidx <= self.size as usize {
            if self.nodes[left_childidx].v < self.nodes[rootidx].v {
                Self::swap_node(&mut self.nodes, rootidx, left_childidx);
                self.heapify_down_min(left_childidx)
            }
        }
    }
}

#[cfg(test)]
pub mod heap_test {
    use crate::heap::Heap;
    use crate::heap::Min;
    use rand::prelude::*;

    #[test]
    fn max_heap_test() {
        const MAX_SIZE: usize = 10001;
        let mut heap = Heap::<u64, MAX_SIZE>::default();
        let mut s = heap.size;
        let mut rng = rand::thread_rng();
        let mut vals: Vec<u64> = vec![];

        for _ in 0..(MAX_SIZE) {
            let n: u64 = rng.gen::<u64>();
            heap._add(n.into());
            vals.push(n.into());
            s += 1;
            assert!(s == heap._size());
        }

        assert_eq!(Some(&heap.nodes[0].v), vals.iter().max());

        for _ in 0..(MAX_SIZE / 2) {
            let old_max = heap.nodes[0].v;
            let index = vals.iter().position(|x| *x == old_max).unwrap();
            vals.remove(index);
            heap._pop();

            let new_max = heap.nodes[0].v;
            assert_eq!(vals.iter().max(), Some(&new_max));
            s -= 1;
            assert!(s == heap._size());
        }
    }
    #[test]
    fn min_heap_test() {
        const MAX_SIZE: usize = 10001;
        let mut heap = Heap::<u64, MAX_SIZE>::default();
        let mut s = heap.size;
        let mut rng = rand::thread_rng();
        let mut vals: Vec<u64> = vec![];

        for _ in 0..(MAX_SIZE) {
            let n: u64 = rng.gen::<u64>();
            heap.add_min(n.into());
            vals.push(n.into());
            s += 1;
            assert!(s == heap._size());
        }

        assert_eq!(Some(&heap.nodes[0].v), vals.iter().min());
    }
}
