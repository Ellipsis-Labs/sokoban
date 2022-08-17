use bytemuck::{Pod, Zeroable};
use core::ops::{Deref, DerefMut, Drop};
use std::cmp::PartialOrd;

#[derive(Debug, Default, Clone, PartialOrd, PartialEq)]
pub struct Node<K, V> {
    pub key: K,
    pub value: V,
}

impl<
        K: PartialOrd + Copy + Clone + Default + Pod + Zeroable,
        V: Copy + Clone + Default + Pod + Zeroable,
    > Copy for Node<K, V>
{
}

unsafe impl<
        K: PartialOrd + Copy + Clone + Default + Pod + Zeroable,
        V: Copy + Clone + Default + Pod + Zeroable,
    > Zeroable for Node<K, V>
{
}

unsafe impl<
        K: PartialOrd + Copy + Clone + Default + Pod + Zeroable,
        V: Copy + Clone + Default + Pod + Zeroable,
    > Pod for Node<K, V>
{
}

#[derive(Debug, Clone)]
pub struct Heap<
    K: PartialOrd + Copy + Clone + Default + Pod + Zeroable,
    V: Copy + Clone + Default + Pod + Zeroable,
    const MAX_SIZE: usize,
> {
    pub size: u64,
    pub nodes: [Node<K, V>; MAX_SIZE],
}

impl<
        K: PartialOrd + Copy + Clone + Default + Pod + Zeroable,
        V: Copy + Clone + Default + Pod + Zeroable,
        const MAX_SIZE: usize,
    > Default for Heap<K, V, MAX_SIZE>
{
    fn default() -> Self {
        Heap {
            size: 0,
            nodes: [Node::default(); MAX_SIZE],
        }
    }
}

impl<
        K: PartialOrd + Copy + Clone + Default + Pod + Zeroable,
        V: Copy + Clone + Default + Pod + Zeroable,
        const MAX_SIZE: usize,
    > Copy for Heap<K, V, MAX_SIZE>
{
}

unsafe impl<
        K: PartialOrd + Copy + Clone + Default + Pod + Zeroable,
        V: Copy + Clone + Default + Pod + Zeroable,
        const MAX_SIZE: usize,
    > Pod for Heap<K, V, MAX_SIZE>
{
}

unsafe impl<
        K: PartialOrd + Copy + Clone + Default + Pod + Zeroable,
        V: Copy + Clone + Default + Pod + Zeroable,
        const MAX_SIZE: usize,
    > Zeroable for Heap<K, V, MAX_SIZE>
{
}

impl<
        K: PartialOrd + Copy + Clone + Default + Pod + Zeroable,
        V: Copy + Clone + Default + Pod + Zeroable,
        const MAX_SIZE: usize,
    > Heap<K, V, MAX_SIZE>
{
    fn swap_node(arr: &mut [Node<K, V>; MAX_SIZE], parent_idx: usize, added_idx: usize) {
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

    pub fn _peek(&self) -> K {
        return self.nodes[0].key;
    }

    fn _heapifyup(&mut self, index: usize) {
        if self.size == 1 {
            return;
        }

        if index == 0 {
            return;
        }

        let mut index: usize = index;
        let mut parent_index = (index - 1) / 2;

        while parent_index >= 0 {
            if self.nodes[index].key > self.nodes[parent_index].key {
                Self::swap_node(&mut self.nodes, index, parent_index);
                if parent_index != 0 {
                    index = parent_index;
                    parent_index = (index - 1) / 2;
                    continue;
                }
                break;
            } else {
                return;
            }
        }
    }

    fn _heapifydown(&mut self, index: usize) {
        let mut parent = index;
        let mut left_childidx = (2 * parent) + 1;
        let mut right_childidx = (2 * parent) + 2;

        while left_childidx <= self.size as usize {
            if right_childidx <= self.size as usize {
                if self.nodes[left_childidx].key > self.nodes[right_childidx].key {
                    if self.nodes[left_childidx].key > self.nodes[parent].key {
                        Self::swap_node(&mut self.nodes, parent, left_childidx);
                        parent = left_childidx;
                        left_childidx = (2 * parent) + 1;
                        right_childidx = (2 * parent) + 2;
                    } else {
                        return;
                    }
                } else if self.nodes[right_childidx].key > self.nodes[left_childidx].key {
                    if self.nodes[right_childidx].key > self.nodes[parent].key {
                        Self::swap_node(&mut self.nodes, parent, right_childidx);
                        parent = right_childidx;
                        left_childidx = (2 * parent) + 1;
                        right_childidx = (2 * parent) + 2;
                    } else {
                        return;
                    }
                }
            } else if left_childidx <= self.size as usize {
                // right doesn't exist, no need to check right
                if self.nodes[left_childidx].key > self.nodes[parent].key {
                    Self::swap_node(&mut self.nodes, parent, left_childidx);
                    parent = left_childidx;
                    left_childidx = (2 * parent) + 1;
                    right_childidx = (2 * parent) + 2;
                } else {
                    return;
                }
            } else {
                return;
            }
        }
    }

    pub fn _push(&mut self, key: K) {
        if self.size as usize == MAX_SIZE - 1 {
            println!("The 'heap is full");
            return;
        }
        let node = Node::<K, V> {
            key: key,
            value: V::default(),
        };
        self.nodes[self.size as usize] = node;
        self._heapifyup((self.size) as usize);
        self.size += 1;
    }

    pub fn _pop(&mut self) -> Option<(K, V)> {
        let k = self.nodes[0].key;
        let v = self.nodes[0].value;
        let lastidx = (self.size - 1) as usize;
        Self::swap_node(&mut self.nodes, 0, lastidx);
        self.nodes[(self.size - 1) as usize] = Node::default();
        self.size -= 1;
        self._heapifydown(0);
        Some((k, v))
    }

    pub fn _push_min(&mut self, value: K) {
        let node = Node::<K, V> {
            key: value,
            value: V::default(),
        };
        self.nodes[self.size as usize] = node;
        self._heapify_up_min(self.size as usize);
        self.size += 1;
    }

    pub fn _pop_min(&mut self) {
        let lastidx = (self.size - 1) as usize;
        Self::swap_node(&mut self.nodes, 0, lastidx);
        self.nodes[(self.size - 1) as usize] = Node::default();
        self.size -= 1;
        self._heapify_down_min(0);
    }

    fn _heapify_up_min(&mut self, index: usize) {
        if self.size == 1 {
            return;
        }
        if index == 0 {
            return;
        }
        let mut index: usize = index;
        let mut parent_index = (index - 1) / 2;

        while parent_index >= 0 {
            if self.nodes[index].key < self.nodes[parent_index].key {
                Self::swap_node(&mut self.nodes, index, parent_index);
                index = parent_index;
                if parent_index != 0 {
                    parent_index = (index - 1) / 2;
                }
            } else {
                return;
            }
        }
    }

    fn _heapify_down_min(&mut self, rootidx: usize) {
        let mut parent = rootidx;
        let mut left_childidx = (2 * rootidx) + 1;
        let mut right_childidx = (2 * rootidx) + 2;

        while left_childidx <= self.size as usize {
            if right_childidx <= self.size as usize {
                if self.nodes[left_childidx].key < self.nodes[right_childidx].key {
                    if self.nodes[left_childidx].key < self.nodes[rootidx].key {
                        Self::swap_node(&mut self.nodes, rootidx, left_childidx);
                        let temp = parent;
                        parent = left_childidx;
                        left_childidx = temp;
                    } else {
                        return;
                    }
                } else if self.nodes[right_childidx].key < self.nodes[left_childidx].key {
                    if self.nodes[right_childidx].key < self.nodes[rootidx].key {
                        Self::swap_node(&mut self.nodes, rootidx, right_childidx);
                        let temp = parent;
                        parent = right_childidx;
                        right_childidx = temp;
                    } else {
                        return;
                    }
                }
            } else if left_childidx <= self.size as usize {
                if self.nodes[left_childidx].key < self.nodes[rootidx].key {
                    Self::swap_node(&mut self.nodes, rootidx, left_childidx);
                    let temp = parent;
                    parent = left_childidx;
                    left_childidx = temp;
                } else {
                    return;
                }
            } else {
                return;
            }
        }
    }

    pub fn peek_mut(&mut self) -> Option<PeekMut<'_, K, V, MAX_SIZE>> {
        if self._is_empty() {
            None
        } else {
            Some(PeekMut {
                heap: self,
                sift: false,
            })
        }
    }

    pub fn _iter(&self) -> BinaryHeapIterator<K, V, MAX_SIZE> {
        BinaryHeapIterator {
            heap: *self,
            current: 0,
        }
    }

    pub fn get_value(&self, index: usize) -> V {
        self.nodes[index].value
    }

    pub fn get_value_mut(&mut self, index: usize) -> &mut V {
        &mut self.nodes[index].value
    }
}

pub struct PeekMut<
    'a,
    K: PartialOrd + Copy + Clone + Default + Pod + Zeroable,
    V: Copy + Clone + Default + Pod + Zeroable,
    const MAX_SIZE: usize,
> {
    heap: &'a mut Heap<K, V, MAX_SIZE>,
    sift: bool,
}

impl<
        K: PartialOrd + Copy + Clone + Default + Pod + Zeroable,
        V: Copy + Clone + Default + Pod + Zeroable,
        const MAX_SIZE: usize,
    > Drop for PeekMut<'_, K, V, MAX_SIZE>
{
    fn drop(&mut self) {
        if self.sift {
            // SAFETY: PeekMut is only instantiated for non-empty heaps.
            {
                self.heap._heapifydown(0)
            };
        }
    }
}

impl<
        K: PartialOrd + Copy + Clone + Default + Pod + Zeroable,
        V: Copy + Clone + Default + Pod + Zeroable,
        const MAX_SIZE: usize,
    > Deref for PeekMut<'_, K, V, MAX_SIZE>
{
    type Target = K;
    fn deref(&self) -> &K {
        debug_assert!(!self.heap._is_empty());
        // SAFE: PeekMut is only instantiated for non-empty heaps
        unsafe { &self.heap.nodes.get_unchecked(0).key }
    }
}

impl<
        K: PartialOrd + Copy + Clone + Default + Pod + Zeroable,
        V: Copy + Clone + Default + Pod + Zeroable,
        const MAX_SIZE: usize,
    > DerefMut for PeekMut<'_, K, V, MAX_SIZE>
{
    fn deref_mut(&mut self) -> &mut K {
        debug_assert!(!self.heap._is_empty());
        self.sift = true;
        // SAFE: PeekMut is only instantiated for non-empty heaps
        unsafe { &mut self.heap.nodes.get_unchecked_mut(0).key }
    }
}

#[derive(Debug)]
pub struct BinaryHeapIterator<
    K: PartialOrd + Copy + Clone + Default + Pod + Zeroable,
    V: Copy + Clone + Default + Pod + Zeroable,
    const MAX_SIZE: usize,
> {
    pub heap: Heap<K, V, MAX_SIZE>,
    pub current: u64,
}
impl<
        K: PartialOrd + Copy + Clone + Default + Pod + Zeroable,
        V: Copy + Clone + Default + Pod + Zeroable,
        const MAX_SIZE: usize,
    > Iterator for BinaryHeapIterator<K, V, MAX_SIZE>
{
    type Item = (K, V);
    fn next(&mut self) -> Option<Self::Item> {
        let mut next: Option<(K, V)> = Some((K::default(), V::default()));
        if self.current < self.heap.size {
            next = Some((
                self.heap.nodes[self.current as usize].key,
                self.heap.nodes[self.current as usize].value,
            ));
            
        }
        self.current += 1;
        next
    }
}