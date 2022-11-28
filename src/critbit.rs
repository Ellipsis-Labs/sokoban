use bytemuck::{Pod, Zeroable};
use std::ops::{Index, IndexMut};

use crate::node_allocator::{
    FromSlice, NodeAllocator, NodeAllocatorMap, OrderedNodeAllocatorMap, TreeField as Field,
    ZeroCopy, SENTINEL,
};

#[repr(C)]
#[derive(Default, Copy, Clone)]
pub struct CritbitNode {
    pub key: u128,
    pub prefix_len: u64,
    pub _padding: u64,
}

unsafe impl Zeroable for CritbitNode {}
unsafe impl Pod for CritbitNode {}

impl CritbitNode {
    pub fn new(prefix_len: u64, key: u128) -> Self {
        Self {
            prefix_len,
            key,
            _padding: 0,
        }
    }
}

#[derive(Copy, Clone)]
pub struct Critbit<
    V: Default + Copy + Clone + Pod + Zeroable,
    const NUM_NODES: usize,
    const MAX_SIZE: usize,
> {
    _padding: u64,
    /// Root node of the critbit tree
    pub root: u64,
    /// Allocator corresponding to inner nodes and leaf pointers of the critbit
    node_allocator: NodeAllocator<CritbitNode, NUM_NODES, 4>,
    /// Allocator corresponding to the leaves of the critbit. Note that this
    /// requires 4 registers per leaf to support proper alignment (for aarch64)
    leaves: NodeAllocator<V, MAX_SIZE, 4>,
}

unsafe impl<V: Default + Copy + Clone + Pod + Zeroable, const NUM_NODES: usize, const MAX_SIZE: usize>
    Zeroable for Critbit<V, NUM_NODES, MAX_SIZE>
{
}

unsafe impl<V: Default + Copy + Clone + Pod + Zeroable, const NUM_NODES: usize, const MAX_SIZE: usize>
    Pod for Critbit<V, NUM_NODES, MAX_SIZE>
{
}

impl<V: Default + Copy + Clone + Pod + Zeroable, const NUM_NODES: usize, const MAX_SIZE: usize>
    ZeroCopy for Critbit<V, NUM_NODES, MAX_SIZE>
{
}

impl<V: Default + Copy + Clone + Pod + Zeroable, const NUM_NODES: usize, const MAX_SIZE: usize>
    Default for Critbit<V, NUM_NODES, MAX_SIZE>
{
    fn default() -> Self {
        assert!(NUM_NODES >= 2 * MAX_SIZE);
        Self {
            _padding: 0,
            root: SENTINEL as u64,
            node_allocator: NodeAllocator::<CritbitNode, NUM_NODES, 4>::default(),
            leaves: NodeAllocator::<V, MAX_SIZE, 4>::default(),
        }
    }
}

impl<V: Default + Copy + Clone + Pod + Zeroable, const NUM_NODES: usize, const MAX_SIZE: usize>
    FromSlice for Critbit<V, NUM_NODES, MAX_SIZE>
{
    fn new_from_slice(slice: &mut [u8]) -> &mut Self {
        assert!(NUM_NODES >= 2 * MAX_SIZE);
        let tree = Self::load_mut_bytes(slice).unwrap();
        tree.initialize();
        tree
    }
}

impl<V: Default + Copy + Clone + Pod + Zeroable, const NUM_NODES: usize, const MAX_SIZE: usize>
    NodeAllocatorMap<u128, V> for Critbit<V, NUM_NODES, MAX_SIZE>
{
    fn insert(&mut self, key: u128, value: V) -> Option<u32> {
        self._insert(key, value)
    }

    fn remove(&mut self, key: &u128) -> Option<V> {
        self._remove(key)
    }

    fn contains(&self, key: &u128) -> bool {
        self.get(key).is_some()
    }

    fn get(&self, key: &u128) -> Option<&V> {
        let mut node_index = self.root as u32;
        loop {
            let node = self.get_node(node_index);
            if !self.is_inner_node(node_index) {
                if node.key == *key {
                    let leaf_index = self.get_leaf_index(node_index);
                    return Some(self.get_leaf(leaf_index));
                } else {
                    return None;
                }
            }
            let shared_prefix_len = (node.key ^ key).leading_zeros() as u64;
            if shared_prefix_len >= node.prefix_len {
                node_index = self.get_child(node.prefix_len, node_index, *key).0;
                continue;
            }
        }
    }

    fn get_mut(&mut self, key: &u128) -> Option<&mut V> {
        let mut node_index = self.root as u32;
        loop {
            let node = self.get_node(node_index);
            if !self.is_inner_node(node_index) {
                if node.key == *key {
                    let leaf_index = self.get_leaf_index(node_index);
                    return Some(self.get_leaf_mut(leaf_index));
                } else {
                    return None;
                }
            }
            let shared_prefix_len = (node.key ^ key).leading_zeros() as u64;
            if shared_prefix_len >= node.prefix_len {
                node_index = self.get_child(node.prefix_len, node_index, *key).0;
                continue;
            }
        }
    }

    fn size(&self) -> usize {
        self.leaves.size as usize
    }

    fn len(&self) -> usize {
        self.leaves.size as usize
    }

    fn capacity(&self) -> usize {
        MAX_SIZE
    }

    fn iter(&self) -> Box<dyn DoubleEndedIterator<Item = (&u128, &V)> + '_> {
        Box::new(self._iter())
    }

    fn iter_mut(&mut self) -> Box<dyn DoubleEndedIterator<Item = (&u128, &mut V)> + '_> {
        Box::new(self._iter_mut())
    }
}

impl<V: Default + Copy + Clone + Pod + Zeroable, const NUM_NODES: usize, const MAX_SIZE: usize>
    OrderedNodeAllocatorMap<u128, V> for Critbit<V, NUM_NODES, MAX_SIZE>
{
    fn get_min_index(&mut self) -> u32 {
        self.find_min(self.root as u32)
    }

    fn get_max_index(&mut self) -> u32 {
        self.find_max(self.root as u32)
    }

    fn get_min(&mut self) -> Option<(u128, V)> {
        match self.get_min_index() {
            SENTINEL => None,
            i => {
                let node = self.get_node(i);
                let leaf = self.get_leaf(self.get_leaf_index(i));
                Some((node.key, *leaf))
            }
        }
    }

    fn get_max(&mut self) -> Option<(u128, V)> {
        match self.get_max_index() {
            SENTINEL => None,
            i => {
                let node = self.get_node(i);
                let leaf = self.get_leaf(self.get_leaf_index(i));
                Some((node.key, *leaf))
            }
        }
    }
}

impl<V: Default + Copy + Clone + Pod + Zeroable, const NUM_NODES: usize, const MAX_SIZE: usize>
    Critbit<V, NUM_NODES, MAX_SIZE>
{
    pub fn new() -> Self {
        Self::default()
    }

    pub fn initialize(&mut self) {
        self.node_allocator.initialize();
        self.leaves.initialize();
    }

    pub fn get_leaf(&self, leaf_index: u32) -> &V {
        self.leaves.get(leaf_index).get_value()
    }

    pub fn get_leaf_mut(&mut self, leaf_index: u32) -> &mut V {
        self.leaves.get_mut(leaf_index).get_value_mut()
    }

    fn get_leaf_index(&self, node: u32) -> u32 {
        self.node_allocator.get_register(node, Field::Value as u32)
    }

    pub fn is_inner_node(&self, node: u32) -> bool {
        self.node_allocator.get_register(node, Field::Value as u32) == SENTINEL
    }

    pub fn get_node(&self, node: u32) -> CritbitNode {
        *self.node_allocator.get(node).get_value()
    }

    pub fn get_key(&self, node: u32) -> &u128 {
        &self.node_allocator.get(node).get_value().key
    }

    #[inline(always)]
    pub fn get_left(&self, node: u32) -> u32 {
        self.node_allocator.get_register(node, Field::Left as u32)
    }

    #[inline(always)]
    pub fn get_right(&self, node: u32) -> u32 {
        self.node_allocator.get_register(node, Field::Right as u32)
    }

    #[inline(always)]
    pub fn get_parent(&self, node: u32) -> u32 {
        self.node_allocator.get_register(node, Field::Parent as u32)
    }

    pub fn get_node_mut(&mut self, node: u32) -> &mut CritbitNode {
        self.node_allocator.get_mut(node).get_value_mut()
    }

    #[inline(always)]
    fn replace_leaf(&mut self, leaf_index: u32, value: V) {
        self.leaves.get_mut(leaf_index).set_value(value);
    }

    #[inline(always)]
    fn add_leaf(&mut self, key: u128, value: V) -> (u32, u32) {
        let node_index = self.node_allocator.add_node(CritbitNode::new(128, key));
        let leaf_index = self.leaves.add_node(value);
        self.node_allocator
            .set_register(node_index, leaf_index, Field::Value as u32);
        self.leaves.get_mut(leaf_index).set_value(value);
        (node_index, leaf_index)
    }

    #[inline(always)]
    fn get_child(&self, prefix_len: u64, node_index: u32, search_key: u128) -> (u32, bool) {
        let crit_bit_mask = (1u128 << 127) >> prefix_len;
        if (search_key & crit_bit_mask) != 0 {
            (self.get_right(node_index), true)
        } else {
            (self.get_left(node_index), false)
        }
    }

    #[inline(always)]
    fn duplicate(&mut self, node_index: u32) -> u32 {
        let index = self.node_allocator.add_node(self.get_node(node_index));
        let left = self.get_left(node_index);
        let right = self.get_right(node_index);
        let value = self
            .node_allocator
            .get_register(node_index, Field::Value as u32);
        self.node_allocator
            .set_register(index, value, Field::Value as u32);
        self.node_allocator
            .connect(index, left, Field::Left as u32, Field::Parent as u32);
        self.node_allocator
            .connect(index, right, Field::Right as u32, Field::Parent as u32);
        index
    }

    #[inline(always)]
    fn replace_node(
        &mut self,
        node_index: u32,
        node_contents: &CritbitNode,
        left: u32,
        right: u32,
    ) {
        *self.get_node_mut(node_index) = *node_contents;
        self.node_allocator
            .clear_register(node_index, Field::Value as u32);
        self.node_allocator
            .connect(node_index, left, Field::Left as u32, Field::Parent as u32);
        self.node_allocator
            .connect(node_index, right, Field::Right as u32, Field::Parent as u32);
    }

    #[inline(always)]
    fn migrate(&mut self, source: u32, target: u32) {
        let content = self.get_node(source);
        *self.get_node_mut(target) = content;
        if !self.is_inner_node(source) {
            assert!(self.get_left(source) == SENTINEL);
            assert!(self.get_right(source) == SENTINEL);
            let leaf_index = self.get_leaf_index(source);
            self.node_allocator
                .clear_register(source, Field::Value as u32);
            self.node_allocator
                .set_register(target, leaf_index, Field::Value as u32);
        }
        assert!(self.get_leaf_index(source) == SENTINEL);
        self.node_allocator.connect(
            target,
            self.get_left(source),
            Field::Left as u32,
            Field::Parent as u32,
        );
        self.node_allocator.connect(
            target,
            self.get_right(source),
            Field::Right as u32,
            Field::Parent as u32,
        );
        self.node_allocator
            .clear_register(source, Field::Left as u32);
        self.node_allocator
            .clear_register(source, Field::Right as u32);
        self.node_allocator.remove_node(source);
    }

    #[inline(always)]
    fn remove_leaf(&mut self, node_index: u32) -> V {
        let leaf_index = self.get_leaf_index(node_index);
        let value = *self.get_leaf(leaf_index);
        self.node_allocator
            .clear_register(node_index, Field::Value as u32);
        assert!(self.get_leaf_index(node_index) == SENTINEL);
        let parent = self.get_parent(node_index);
        if node_index == self.get_left(parent) {
            self.node_allocator.disconnect(
                node_index,
                parent,
                Field::Parent as u32,
                Field::Left as u32,
            );
        } else if node_index == self.get_right(parent) {
            self.node_allocator.disconnect(
                node_index,
                parent,
                Field::Parent as u32,
                Field::Right as u32,
            );
        } else if parent != SENTINEL {
            panic!("Parent is not connected to child");
        }
        self.leaves.remove_node(leaf_index);
        self.node_allocator.remove_node(node_index);
        value
    }

    pub fn get_addr(&self, key: u128) -> u32 {
        let mut node_index = self.root as u32;
        loop {
            let node = self.get_node(node_index);
            if !self.is_inner_node(node_index) {
                if node.key == key {
                    return node_index;
                } else {
                    return SENTINEL;
                }
            }
            let shared_prefix_len = (node.key ^ key).leading_zeros() as u64;
            if shared_prefix_len >= node.prefix_len {
                node_index = self.get_child(node.prefix_len, node_index, key).0;
                continue;
            }
        }
    }

    fn _insert(&mut self, key: u128, value: V) -> Option<u32> {
        if self.root as u32 == SENTINEL {
            let (node_index, _leaf_index) = self.add_leaf(key, value);
            self.root = node_index as u64;
            return Some(self.root as u32);
        }
        // Return None if the tree is filled up
        if self.len() >= self.capacity() {
            return None;
        }
        let mut node_index = self.root as u32;
        loop {
            let node = self.get_node(node_index);
            if node.key == key && !self.is_inner_node(node_index) {
                // Replace the node with the new value
                let leaf_index = self.get_leaf_index(node_index);
                self.replace_leaf(leaf_index, value);
                return Some(node_index);
            }
            let shared_prefix_len = (node.key ^ key).leading_zeros() as u64;
            if shared_prefix_len >= node.prefix_len {
                node_index = self.get_child(node.prefix_len, node_index, key).0;
                continue;
            }
            let crit_bit_mask: u128 = (1u128 << 127) >> shared_prefix_len;
            let is_right = (crit_bit_mask & key) != 0;
            let (node_leaf_index, _leaf_index) = self.add_leaf(key, value);
            let moved_node_index = self.duplicate(node_index);
            let new_node = CritbitNode::new(shared_prefix_len, key);
            if is_right {
                self.replace_node(node_index, &new_node, moved_node_index, node_leaf_index);
            } else {
                self.replace_node(node_index, &new_node, node_leaf_index, moved_node_index);
            }
            return Some(node_leaf_index);
        }
    }

    fn _remove(&mut self, key: &u128) -> Option<V> {
        let nsize = self.node_allocator.size;
        let lsize = self.leaves.size;
        let mut parent = self.root as u32;
        let mut child: u32;
        let mut is_right: bool;
        if self.len() == 0 {
            return None;
        }
        if self.is_inner_node(parent) {
            let node = self.get_node(parent);
            let (c, ir) = self.get_child(node.prefix_len, parent, *key);
            child = c;
            is_right = ir;
        } else {
            let leaf = self.get_node(parent);
            if leaf.key == *key {
                self.root = SENTINEL as u64;
                assert!(self.len() == 1);
                return Some(self.remove_leaf(parent));
            } else {
                return None;
            }
        }
        loop {
            let node = self.get_node(child);
            if self.is_inner_node(child) {
                let (grandchild, grandchild_crit_bit) =
                    self.get_child(node.prefix_len, child, *key);
                parent = child;
                child = grandchild;
                is_right = grandchild_crit_bit;
            } else {
                if node.key != *key {
                    return None;
                }
                break;
            }
        }
        let sibling = if is_right {
            self.get_left(parent)
        } else {
            self.get_right(parent)
        };
        let leaf = self.remove_leaf(child);
        self.migrate(sibling, parent);
        assert!(nsize - self.node_allocator.size == 2);
        assert!(lsize - self.leaves.size == 1);
        Some(leaf)
    }

    fn find_min(&self, index: u32) -> u32 {
        let mut node = index;
        while self.get_left(node) != SENTINEL {
            node = self.get_left(node);
        }
        node
    }

    fn find_max(&self, index: u32) -> u32 {
        let mut node = index;
        while self.get_right(node) != SENTINEL {
            node = self.get_right(node);
        }
        node
    }

    fn _iter(&self) -> CritbitIterator<'_, V, NUM_NODES, MAX_SIZE> {
        CritbitIterator::<V, NUM_NODES, MAX_SIZE> {
            tree: self,
            stack: vec![self.root as u32],
            rev_stack: vec![self.root as u32],
        }
    }

    fn _iter_mut(&mut self) -> CritbitIteratorMut<'_, V, NUM_NODES, MAX_SIZE> {
        let node = self.root as u32;
        CritbitIteratorMut::<V, NUM_NODES, MAX_SIZE> {
            tree: self,
            stack: vec![node],
            rev_stack: vec![node],
        }
    }
}

impl<
        'a,
        V: Default + Copy + Clone + Pod + Zeroable,
        const MAX_NODES: usize,
        const MAX_SIZE: usize,
    > IntoIterator for &'a Critbit<V, MAX_NODES, MAX_SIZE>
{
    type Item = (&'a u128, &'a V);
    type IntoIter = CritbitIterator<'a, V, MAX_NODES, MAX_SIZE>;

    fn into_iter(self) -> Self::IntoIter {
        self._iter()
    }
}

impl<
        'a,
        V: Default + Copy + Clone + Pod + Zeroable,
        const MAX_NODES: usize,
        const MAX_SIZE: usize,
    > IntoIterator for &'a mut Critbit<V, MAX_NODES, MAX_SIZE>
{
    type Item = (&'a u128, &'a mut V);
    type IntoIter = CritbitIteratorMut<'a, V, MAX_NODES, MAX_SIZE>;

    fn into_iter(self) -> Self::IntoIter {
        self._iter_mut()
    }
}

pub struct CritbitIterator<
    'a,
    V: Default + Copy + Clone + Pod + Zeroable,
    const MAX_NODES: usize,
    const MAX_SIZE: usize,
> {
    tree: &'a Critbit<V, MAX_NODES, MAX_SIZE>,
    stack: Vec<u32>,
    rev_stack: Vec<u32>,
}

impl<
        'a,
        V: Default + Copy + Clone + Pod + Zeroable,
        const MAX_NODES: usize,
        const MAX_SIZE: usize,
    > Iterator for CritbitIterator<'a, V, MAX_NODES, MAX_SIZE>
{
    type Item = (&'a u128, &'a V);

    fn next(&mut self) -> Option<Self::Item> {
        while !self.stack.is_empty() {
            let node = self.stack.pop();
            match node {
                Some(n) => {
                    if !self.tree.is_inner_node(n) {
                        let i = self.tree.get_leaf_index(n);
                        let v = self.tree.get_leaf(i);
                        let k = self.tree.get_key(n);
                        return Some((k, v));
                    } else {
                        self.stack.push(self.tree.get_right(n));
                        self.stack.push(self.tree.get_left(n));
                    }
                }
                _ => return None,
            }
        }
        None
    }
}

impl<
        'a,
        V: Default + Copy + Clone + Pod + Zeroable,
        const MAX_NODES: usize,
        const MAX_SIZE: usize,
    > DoubleEndedIterator for CritbitIterator<'a, V, MAX_NODES, MAX_SIZE>
{
    fn next_back(&mut self) -> Option<Self::Item> {
        while !self.rev_stack.is_empty() {
            let node = self.rev_stack.pop();
            match node {
                Some(n) => {
                    if !self.tree.is_inner_node(n) {
                        let i = self.tree.get_leaf_index(n);
                        let v = self.tree.get_leaf(i);
                        let k = self.tree.get_key(n);
                        return Some((k, v));
                    } else {
                        self.rev_stack.push(self.tree.get_left(n));
                        self.rev_stack.push(self.tree.get_right(n));
                    }
                }
                _ => return None,
            }
        }
        None
    }
}

pub struct CritbitIteratorMut<
    'a,
    V: Default + Copy + Clone + Pod + Zeroable,
    const MAX_NODES: usize,
    const MAX_SIZE: usize,
> {
    tree: &'a mut Critbit<V, MAX_NODES, MAX_SIZE>,
    stack: Vec<u32>,
    rev_stack: Vec<u32>,
}

impl<
        'a,
        V: Default + Copy + Clone + Pod + Zeroable,
        const MAX_NODES: usize,
        const MAX_SIZE: usize,
    > Iterator for CritbitIteratorMut<'a, V, MAX_NODES, MAX_SIZE>
{
    type Item = (&'a u128, &'a mut V);

    fn next(&mut self) -> Option<Self::Item> {
        while !self.stack.is_empty() {
            let node = self.stack.pop();
            match node {
                Some(n) => {
                    if !self.tree.is_inner_node(n) {
                        let i = self.tree.get_leaf_index(n);
                        unsafe {
                            let key = &(*self
                                .tree
                                .node_allocator
                                .nodes
                                .as_ptr()
                                .add((n - 1) as usize))
                            .get_value()
                            .key;
                            let leaf = (*self.tree.leaves.nodes.as_mut_ptr().add((i - 1) as usize))
                                .get_value_mut();
                            return Some((key, leaf));
                        }
                    } else {
                        self.stack.push(self.tree.get_right(n));
                        self.stack.push(self.tree.get_left(n));
                    }
                }
                _ => return None,
            }
        }
        None
    }
}

impl<
        'a,
        V: Default + Copy + Clone + Pod + Zeroable,
        const MAX_NODES: usize,
        const MAX_SIZE: usize,
    > DoubleEndedIterator for CritbitIteratorMut<'a, V, MAX_NODES, MAX_SIZE>
{
    fn next_back(&mut self) -> Option<Self::Item> {
        while !self.rev_stack.is_empty() {
            let node = self.rev_stack.pop();
            match node {
                Some(n) => {
                    if !self.tree.is_inner_node(n) {
                        let i = self.tree.get_leaf_index(n);
                        unsafe {
                            let key = &(*self
                                .tree
                                .node_allocator
                                .nodes
                                .as_ptr()
                                .add((n - 1) as usize))
                            .get_value()
                            .key;
                            let leaf = (*self.tree.leaves.nodes.as_mut_ptr().add((i - 1) as usize))
                                .get_value_mut();
                            return Some((key, leaf));
                        }
                    } else {
                        self.rev_stack.push(self.tree.get_left(n));
                        self.rev_stack.push(self.tree.get_right(n));
                    }
                }
                _ => return None,
            }
        }
        None
    }
}

impl<V: Default + Copy + Clone + Pod + Zeroable, const NUM_NODES: usize, const MAX_SIZE: usize>
    Index<u128> for Critbit<V, NUM_NODES, MAX_SIZE>
{
    type Output = V;

    fn index(&self, index: u128) -> &Self::Output {
        self.get(&index).unwrap()
    }
}

impl<V: Default + Copy + Clone + Pod + Zeroable, const NUM_NODES: usize, const MAX_SIZE: usize>
    IndexMut<u128> for Critbit<V, NUM_NODES, MAX_SIZE>
{
    fn index_mut(&mut self, index: u128) -> &mut Self::Output {
        self.get_mut(&index).unwrap()
    }
}
