use bytemuck::{Pod, Zeroable};
use num_derive::FromPrimitive;
use num_traits::FromPrimitive;
use std::ops::{Index, IndexMut};

use crate::node_allocator::{
    FromSlice, NodeAllocator, NodeAllocatorMap, OrderedNodeAllocatorMap, TreeField as Field,
    ZeroCopy, SENTINEL,
};

pub const ALIGNMENT: u32 = 8;

// Register aliases
pub const COLOR: u32 = Field::Value as u32;

#[derive(Debug, Copy, Clone, PartialEq, Eq, FromPrimitive)]
pub enum Color {
    Black = 0,
    Red = 1,
}

/// Exploits the fact that LEFT and RIGHT are set to 0 and 1 respectively
#[inline(always)]
fn opposite(dir: u32) -> u32 {
    1 - dir
}

#[repr(C)]
#[derive(Default, Copy, Clone)]
pub struct RBNode<
    K: PartialOrd + Copy + Clone + Default + Pod + Zeroable,
    V: Default + Copy + Clone + Pod + Zeroable,
> {
    pub key: K,
    pub value: V,
}

unsafe impl<
        K: PartialOrd + Copy + Clone + Default + Pod + Zeroable,
        V: Default + Copy + Clone + Pod + Zeroable,
    > Zeroable for RBNode<K, V>
{
}
unsafe impl<
        K: PartialOrd + Copy + Clone + Default + Pod + Zeroable,
        V: Default + Copy + Clone + Pod + Zeroable,
    > Pod for RBNode<K, V>
{
}

impl<
        K: PartialOrd + Copy + Clone + Default + Pod + Zeroable,
        V: Default + Copy + Clone + Pod + Zeroable,
    > RBNode<K, V>
{
    pub fn new(key: K, value: V) -> Self {
        Self { key, value }
    }
}

#[derive(Copy, Clone)]
pub struct RedBlackTree<
    K: PartialOrd + Copy + Clone + Default + Pod + Zeroable,
    V: Default + Copy + Clone + Pod + Zeroable,
    const MAX_SIZE: usize,
> {
    pub root: u64,
    _padding: u64,
    allocator: NodeAllocator<RBNode<K, V>, MAX_SIZE, 4>,
}

unsafe impl<
        K: PartialOrd + Copy + Clone + Default + Pod + Zeroable,
        V: Default + Copy + Clone + Pod + Zeroable,
        const MAX_SIZE: usize,
    > Zeroable for RedBlackTree<K, V, MAX_SIZE>
{
}
unsafe impl<
        K: PartialOrd + Copy + Clone + Default + Pod + Zeroable,
        V: Default + Copy + Clone + Pod + Zeroable,
        const MAX_SIZE: usize,
    > Pod for RedBlackTree<K, V, MAX_SIZE>
{
}

impl<
        K: PartialOrd + Copy + Clone + Default + Pod + Zeroable,
        V: Default + Copy + Clone + Pod + Zeroable,
        const MAX_SIZE: usize,
    > ZeroCopy for RedBlackTree<K, V, MAX_SIZE>
{
}

impl<
        K: PartialOrd + Copy + Clone + Default + Pod + Zeroable,
        V: Default + Copy + Clone + Pod + Zeroable,
        const MAX_SIZE: usize,
    > Default for RedBlackTree<K, V, MAX_SIZE>
{
    fn default() -> Self {
        Self::assert_proper_alignment();
        RedBlackTree {
            root: SENTINEL as u64,
            _padding: 0,
            allocator: NodeAllocator::<RBNode<K, V>, MAX_SIZE, 4>::default(),
        }
    }
}

impl<
        K: PartialOrd + Copy + Clone + Default + Pod + Zeroable,
        V: Default + Copy + Clone + Pod + Zeroable,
        const MAX_SIZE: usize,
    > FromSlice for RedBlackTree<K, V, MAX_SIZE>
{
    fn new_from_slice(slice: &mut [u8]) -> &mut Self {
        Self::assert_proper_alignment();
        let tree = Self::load_mut_bytes(slice).unwrap();
        tree.initialize();
        tree
    }
}

impl<
        K: PartialOrd + Copy + Clone + Default + Pod + Zeroable,
        V: Default + Copy + Clone + Pod + Zeroable,
        const MAX_SIZE: usize,
    > NodeAllocatorMap<K, V> for RedBlackTree<K, V, MAX_SIZE>
{
    fn insert(&mut self, key: K, value: V) -> Option<u32> {
        self._insert(key, value)
    }

    fn remove(&mut self, key: &K) -> Option<V> {
        self._remove(key)
    }

    fn contains(&self, key: &K) -> bool {
        self.get(key).is_some()
    }

    fn get(&self, key: &K) -> Option<&V> {
        let mut reference_node = self.root as u32;
        if reference_node == SENTINEL {
            return None;
        }
        loop {
            let ref_value = self.allocator.get(reference_node).get_value().key;
            let target = if *key < ref_value {
                self.get_left(reference_node)
            } else if *key > ref_value {
                self.get_right(reference_node)
            } else {
                return Some(&self.get_node(reference_node).value);
            };
            if target == SENTINEL {
                return None;
            }
            reference_node = target
        }
    }

    fn get_mut(&mut self, key: &K) -> Option<&mut V> {
        let mut reference_node = self.root as u32;
        if reference_node == SENTINEL {
            return None;
        }
        loop {
            let ref_value = self.allocator.get(reference_node).get_value().key;
            let target = if *key < ref_value {
                self.get_left(reference_node)
            } else if *key > ref_value {
                self.get_right(reference_node)
            } else {
                return Some(&mut self.get_node_mut(reference_node).value);
            };
            if target == SENTINEL {
                return None;
            }
            reference_node = target
        }
    }

    fn size(&self) -> usize {
        self.allocator.size as usize
    }
    fn iter(&self) -> Box<dyn DoubleEndedIterator<Item = (&K, &V)> + '_> {
        Box::new(self._iter())
    }

    fn iter_mut(&mut self) -> Box<dyn DoubleEndedIterator<Item = (&K, &mut V)> + '_> {
        Box::new(self._iter_mut())
    }
}

impl<
        K: PartialOrd + Copy + Clone + Default + Pod + Zeroable,
        V: Default + Copy + Clone + Pod + Zeroable,
        const MAX_SIZE: usize,
    > OrderedNodeAllocatorMap<K, V> for RedBlackTree<K, V, MAX_SIZE>
{
    fn get_min_index(&mut self) -> u32 {
        self.find_min(self.root as u32)
    }

    fn get_max_index(&mut self) -> u32 {
        self.find_max(self.root as u32)
    }

    fn get_min(&mut self) -> Option<(K, V)> {
        match self.get_min_index() {
            SENTINEL => None,
            i => {
                let node = self.get_node(i);
                Some((node.key, node.value))
            }
        }
    }

    fn get_max(&mut self) -> Option<(K, V)> {
        match self.get_max_index() {
            SENTINEL => None,
            i => {
                let node = self.get_node(i);
                Some((node.key, node.value))
            }
        }
    }
}

impl<
        K: PartialOrd + Copy + Clone + Default + Pod + Zeroable,
        V: Default + Copy + Clone + Pod + Zeroable,
        const MAX_SIZE: usize,
    > RedBlackTree<K, V, MAX_SIZE>
{
    pub fn print_node(&self, node: u32)
    where
        K: std::fmt::Debug,
        V: std::fmt::Debug,
    {
        println!("Node Index: {}", node);
        println!("Left: {}", self.get_left(node));
        println!("Right: {}", self.get_right(node));
        println!("Parent: {}", self.get_parent(node));
        println!(
            "Color: {}",
            if self.is_black(node) { "Black" } else { "Red" }
        );
        println!("Key: {:?}", self.get_node(node).key);
        println!()
    }

    fn assert_proper_alignment() {
        // TODO is this a sufficient coverage of the edge cases?
        assert!(std::mem::size_of::<V>() % std::mem::align_of::<K>() == 0);
        assert!(std::mem::size_of::<RBNode<K, V>>() % std::mem::align_of::<RBNode<K, V>>() == 0);
        assert!(std::mem::size_of::<RBNode<K, V>>() % 8_usize == 0);
    }

    pub fn new() -> Self {
        Self::default()
    }

    #[inline(always)]
    pub fn initialize(&mut self) {
        self.allocator.initialize();
    }

    pub fn get_node(&self, node: u32) -> &RBNode<K, V> {
        self.allocator.get(node).get_value()
    }

    pub fn get_node_mut(&mut self, node: u32) -> &mut RBNode<K, V> {
        self.allocator.get_mut(node).get_value_mut()
    }

    #[inline(always)]
    fn color_red(&mut self, node: u32) {
        if node != SENTINEL {
            self.allocator.set_register(node, Color::Red as u32, COLOR);
        }
    }

    #[inline(always)]
    fn color_black(&mut self, node: u32) {
        self.allocator
            .set_register(node, Color::Black as u32, COLOR);
    }

    #[inline(always)]
    fn is_red(&self, node: u32) -> bool {
        self.allocator.get_register(node, COLOR) == Color::Red as u32
    }

    #[inline(always)]
    fn is_black(&self, node: u32) -> bool {
        self.allocator.get_register(node, COLOR) == Color::Black as u32
    }

    #[inline(always)]
    fn get_child(&self, node: u32, dir: u32) -> u32 {
        self.allocator.get_register(node, dir)
    }

    #[inline(always)]
    pub fn is_leaf(&self, node: u32) -> bool {
        self.get_left(node) == SENTINEL && self.get_right(node) == SENTINEL
    }

    #[inline(always)]
    pub fn get_left(&self, node: u32) -> u32 {
        self.allocator.get_register(node, Field::Left as u32)
    }

    #[inline(always)]
    pub fn get_right(&self, node: u32) -> u32 {
        self.allocator.get_register(node, Field::Right as u32)
    }

    #[inline(always)]
    pub fn get_color(&self, node: u32) -> u32 {
        self.allocator.get_register(node, COLOR)
    }

    #[inline(always)]
    pub fn get_parent(&self, node: u32) -> u32 {
        self.allocator.get_register(node, Field::Parent as u32)
    }

    #[inline(always)]
    fn connect(&mut self, parent: u32, child: u32, dir: u32) {
        self.allocator
            .connect(parent, child, dir, Field::Parent as u32);
    }

    #[inline(always)]
    fn child_dir(&self, parent: u32, child: u32) -> u32 {
        let left = self.get_left(parent);
        let right = self.get_right(parent);
        if child == left {
            Field::Left as u32
        } else if child == right {
            Field::Right as u32
        } else {
            panic!("Nodes are not connected");
        }
    }

    fn rotate_dir(&mut self, parent_index: u32, dir: u32) -> Option<u32> {
        let grandparent_index = self.get_parent(parent_index);
        match FromPrimitive::from_u32(dir) {
            Some(Field::Left) | Some(Field::Right) => {}
            _ => return None,
        }
        let sibling_index = self.get_child(parent_index, opposite(dir));
        if sibling_index == SENTINEL {
            return None;
        }
        let child_index = self.get_child(sibling_index, dir);
        self.connect(sibling_index, parent_index, dir);
        self.connect(parent_index, child_index, opposite(dir));
        if grandparent_index != SENTINEL {
            if self.get_left(grandparent_index) == parent_index {
                self.connect(grandparent_index, sibling_index, Field::Left as u32);
            } else if self.get_right(grandparent_index) == parent_index {
                self.connect(grandparent_index, sibling_index, Field::Right as u32);
            } else {
                return None;
            }
        } else {
            self.allocator
                .clear_register(sibling_index, Field::Parent as u32);
            self.root = sibling_index as u64;
        }
        Some(sibling_index)
    }

    fn fix_insert(&mut self, mut node: u32) -> Option<()> {
        while self.is_red(self.get_parent(node)) {
            let mut parent = self.get_parent(node);
            let mut grandparent = self.get_parent(parent);
            if grandparent == SENTINEL {
                assert!(parent == self.root as u32);
                break;
            }
            let dir = self.child_dir(grandparent, parent);
            let uncle = self.get_child(grandparent, opposite(dir));
            if self.is_red(uncle) {
                self.color_black(uncle);
                self.color_black(parent);
                self.color_red(grandparent);
                node = grandparent;
            } else {
                if self.child_dir(parent, node) == opposite(dir) {
                    self.rotate_dir(parent, dir);
                    node = parent;
                }
                parent = self.get_parent(node);
                grandparent = self.get_parent(parent);
                self.color_black(parent);
                self.color_red(grandparent);
                self.rotate_dir(grandparent, opposite(dir));
            }
        }
        self.color_black(self.root as u32);
        Some(())
    }

    fn _insert(&mut self, key: K, value: V) -> Option<u32> {
        let mut reference_node = self.root as u32;
        let new_node = RBNode::<K, V>::new(key, value);
        if reference_node == SENTINEL {
            let node_index = self.allocator.add_node(new_node);
            self.root = node_index as u64;
            return Some(node_index);
        }
        loop {
            let ref_value = self.get_node(reference_node).key;
            let (target, dir) = if key < ref_value {
                (self.get_left(reference_node), Field::Left as u32)
            } else if key > ref_value {
                (self.get_right(reference_node), Field::Right as u32)
            } else {
                self.get_node_mut(reference_node).value = value;
                return Some(reference_node);
            };
            if target == SENTINEL {
                if self.size() >= MAX_SIZE {
                    return None;
                }
                let node_index = self.allocator.add_node(new_node);
                self.color_red(node_index);
                self.connect(reference_node, node_index, dir);
                let grandparent = self.get_parent(reference_node);
                // This is only false when the parent is the root
                if grandparent != SENTINEL {
                    self.fix_insert(node_index);
                }
                return Some(node_index);
            }
            reference_node = target
        }
    }

    fn fix_remove(&mut self, mut node_index: u32) -> Option<()> {
        if node_index == SENTINEL {
            return Some(());
        }
        while node_index != self.root as u32 && self.is_black(node_index) {
            let parent = self.get_parent(node_index);
            let dir = self.child_dir(parent, node_index);
            let mut sibling = self.get_child(parent, opposite(dir));
            if self.is_red(sibling) {
                self.color_black(sibling);
                self.color_red(parent);
                self.rotate_dir(parent, dir);
                sibling = self.get_right(self.get_parent(node_index));
            }
            if self.is_black(self.get_left(sibling)) && self.is_black(self.get_right(sibling)) {
                self.color_red(sibling);
                node_index = self.get_parent(node_index);
            } else {
                if self.is_black(self.get_right(sibling)) {
                    self.color_black(self.get_left(sibling));
                    self.color_red(sibling);
                    self.rotate_dir(sibling, opposite(dir));
                    sibling = self.get_right(self.get_parent(node_index));
                }

                let parent = self.get_parent(node_index);
                if self.is_red(parent) {
                    self.color_red(sibling);
                } else {
                    self.color_black(sibling);
                }
                self.color_black(parent);
                self.color_black(self.get_right(sibling));
                self.rotate_dir(parent, dir);
                node_index = self.root as u32;
            }
        }
        Some(())
    }

    fn _remove(&mut self, key: &K) -> Option<V> {
        let mut ref_node_index = self.root as u32;
        if ref_node_index == SENTINEL {
            return None;
        }
        loop {
            let ref_key = self.allocator.get(ref_node_index).get_value().key;
            let ref_value = self.allocator.get(ref_node_index).get_value().value;
            let left = self.get_left(ref_node_index);
            let right = self.get_right(ref_node_index);
            let target = if *key < ref_key {
                left
            } else if *key > ref_key {
                right
            } else {
                let mut is_black = self.is_black(ref_node_index);
                let (pivot_node_index, delete_node_index) = if left == SENTINEL {
                    self.transplant(ref_node_index, right);
                    // After this step, the node to be deleted has no more childre
                    self.allocator
                        .clear_register(ref_node_index, Field::Right as u32);
                    (right, ref_node_index)
                } else if right == SENTINEL {
                    self.transplant(ref_node_index, left);
                    // After this step, the node to be deleted has no more childre
                    self.allocator
                        .clear_register(ref_node_index, Field::Left as u32);
                    (left, ref_node_index)
                } else {
                    let min_right = self.find_min(right);
                    let min_right_child = self.get_right(min_right);
                    is_black = self.is_black(min_right);
                    if min_right == right {
                        assert!(
                            min_right_child == SENTINEL
                                || self.get_parent(min_right_child) == right
                        );
                    } else {
                        self.transplant(min_right, min_right_child);
                        self.connect(min_right, right, Field::Right as u32);
                    }
                    self.transplant(ref_node_index, min_right);
                    self.connect(min_right, left, Field::Left as u32);
                    if self.is_red(ref_node_index) {
                        self.color_red(min_right)
                    } else {
                        self.color_black(min_right)
                    }
                    self.allocator
                        .clear_register(ref_node_index, Field::Left as u32);
                    self.allocator
                        .clear_register(ref_node_index, Field::Right as u32);
                    (min_right_child, ref_node_index)
                };
                // The parent register and color register of the removed node are cleared
                self.allocator
                    .clear_register(ref_node_index, Field::Parent as u32);
                self.allocator.clear_register(delete_node_index, COLOR);
                // The removed node is added to the free list
                self.allocator.remove_node(delete_node_index);
                // This condition will short circuit if the removed node is red
                // Otherwise, the fixup function will be called to restore the
                // red-black tree properties
                if is_black && self.fix_remove(pivot_node_index).is_none() {
                    return None;
                }
                return Some(ref_value);
            };
            if target == SENTINEL {
                return None;
            }
            ref_node_index = target
        }
    }

    #[inline(always)]
    /// This helper function connects the parent of `target` to `source`.
    /// It is the start of the process of removing `target` from the tree.
    fn transplant(&mut self, target: u32, source: u32) {
        let parent = self.get_parent(target);
        if parent == SENTINEL {
            self.root = source as u64;
            self.allocator
                .set_register(source, SENTINEL, Field::Parent as u32);
            return;
        }
        let dir = self.child_dir(parent, target);
        self.connect(parent, source, dir);
    }

    pub fn get_addr(&self, key: &K) -> u32 {
        let mut reference_node = self.root as u32;
        if reference_node == SENTINEL {
            return SENTINEL;
        }
        loop {
            let ref_value = self.allocator.get(reference_node).get_value().key;
            let target = if *key < ref_value {
                self.get_left(reference_node)
            } else if *key > ref_value {
                self.get_right(reference_node)
            } else {
                return reference_node;
            };
            if target == SENTINEL {
                return SENTINEL;
            }
            reference_node = target
        }
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

    fn _iter(&self) -> RedBlackTreeIterator<'_, K, V, MAX_SIZE> {
        RedBlackTreeIterator::<K, V, MAX_SIZE> {
            tree: self,
            stack: vec![],
            rev_stack: vec![],
            node: self.root as u32,
        }
    }

    fn _iter_mut(&mut self) -> RedBlackTreeIteratorMut<'_, K, V, MAX_SIZE> {
        let node = self.root as u32;
        RedBlackTreeIteratorMut::<K, V, MAX_SIZE> {
            tree: self,
            stack: vec![],
            rev_stack: vec![],
            node,
        }
    }
}

impl<
        'a,
        K: PartialOrd + Copy + Clone + Default + Pod + Zeroable,
        V: Default + Copy + Clone + Pod + Zeroable,
        const MAX_SIZE: usize,
    > IntoIterator for &'a RedBlackTree<K, V, MAX_SIZE>
{
    type Item = (&'a K, &'a V);
    type IntoIter = RedBlackTreeIterator<'a, K, V, MAX_SIZE>;
    fn into_iter(self) -> Self::IntoIter {
        self._iter()
    }
}

impl<
        'a,
        K: PartialOrd + Copy + Clone + Default + Pod + Zeroable,
        V: Default + Copy + Clone + Pod + Zeroable,
        const MAX_SIZE: usize,
    > IntoIterator for &'a mut RedBlackTree<K, V, MAX_SIZE>
{
    type Item = (&'a K, &'a mut V);
    type IntoIter = RedBlackTreeIteratorMut<'a, K, V, MAX_SIZE>;
    fn into_iter(self) -> Self::IntoIter {
        self._iter_mut()
    }
}

pub struct RedBlackTreeIterator<
    'a,
    K: PartialOrd + Copy + Clone + Default + Pod + Zeroable,
    V: Default + Copy + Clone + Pod + Zeroable,
    const MAX_SIZE: usize,
> {
    tree: &'a RedBlackTree<K, V, MAX_SIZE>,
    stack: Vec<u32>,
    rev_stack: Vec<u32>,
    node: u32,
}

impl<
        'a,
        K: PartialOrd + Copy + Clone + Default + Pod + Zeroable,
        V: Default + Copy + Clone + Pod + Zeroable,
        const MAX_SIZE: usize,
    > Iterator for RedBlackTreeIterator<'a, K, V, MAX_SIZE>
{
    type Item = (&'a K, &'a V);

    fn next(&mut self) -> Option<Self::Item> {
        while !self.stack.is_empty() || self.node != SENTINEL {
            if self.node != SENTINEL {
                self.stack.push(self.node);
                self.node = self.tree.get_left(self.node);
            } else {
                self.node = self.stack.pop().unwrap();
                let node = self.tree.get_node(self.node);
                self.node = self.tree.get_right(self.node);
                return Some((&node.key, &node.value));
            }
        }
        None
    }
}

impl<
        'a,
        K: PartialOrd + Copy + Clone + Default + Pod + Zeroable,
        V: Default + Copy + Clone + Pod + Zeroable,
        const MAX_SIZE: usize,
    > DoubleEndedIterator for RedBlackTreeIterator<'a, K, V, MAX_SIZE>
{
    fn next_back(&mut self) -> Option<Self::Item> {
        while !self.rev_stack.is_empty() || self.node != SENTINEL {
            if self.node != SENTINEL {
                self.rev_stack.push(self.node);
                self.node = self.tree.get_right(self.node);
            } else {
                self.node = self.rev_stack.pop().unwrap();
                let node = self.tree.get_node(self.node);
                self.node = self.tree.get_left(self.node);
                return Some((&node.key, &node.value));
            }
        }
        None
    }
}

pub struct RedBlackTreeIteratorMut<
    'a,
    K: PartialOrd + Copy + Clone + Default + Pod + Zeroable,
    V: Default + Copy + Clone + Pod + Zeroable,
    const MAX_SIZE: usize,
> {
    tree: &'a mut RedBlackTree<K, V, MAX_SIZE>,
    stack: Vec<u32>,
    rev_stack: Vec<u32>,
    node: u32,
}

impl<
        'a,
        K: PartialOrd + Copy + Clone + Default + Pod + Zeroable,
        V: Default + Copy + Clone + Pod + Zeroable,
        const MAX_SIZE: usize,
    > Iterator for RedBlackTreeIteratorMut<'a, K, V, MAX_SIZE>
{
    type Item = (&'a K, &'a mut V);

    fn next(&mut self) -> Option<Self::Item> {
        while !self.stack.is_empty() || self.node != SENTINEL {
            if self.node != SENTINEL {
                self.stack.push(self.node);
                self.node = self.tree.get_left(self.node);
            } else {
                self.node = self.stack.pop().unwrap();
                let ptr = self.node;
                self.node = self.tree.get_right(ptr);
                // TODO: How does one remove this unsafe?
                unsafe {
                    let node = (*self
                        .tree
                        .allocator
                        .nodes
                        .as_mut_ptr()
                        .add((ptr - 1) as usize))
                    .get_value_mut();
                    return Some((&node.key, &mut node.value));
                }
            }
        }
        None
    }
}

impl<
        'a,
        K: PartialOrd + Copy + Clone + Default + Pod + Zeroable,
        V: Default + Copy + Clone + Pod + Zeroable,
        const MAX_SIZE: usize,
    > DoubleEndedIterator for RedBlackTreeIteratorMut<'a, K, V, MAX_SIZE>
{
    fn next_back(&mut self) -> Option<Self::Item> {
        while !self.stack.is_empty() || self.node != SENTINEL {
            if self.node != SENTINEL {
                self.rev_stack.push(self.node);
                self.node = self.tree.get_right(self.node);
            } else {
                self.node = self.rev_stack.pop().unwrap();
                let ptr = self.node;
                self.node = self.tree.get_left(ptr);
                // TODO: How does one remove this unsafe?
                unsafe {
                    let node = (*self
                        .tree
                        .allocator
                        .nodes
                        .as_mut_ptr()
                        .add((ptr - 1) as usize))
                    .get_value_mut();
                    return Some((&node.key, &mut node.value));
                }
            }
        }
        None
    }
}

impl<
        K: PartialOrd + Copy + Clone + Default + Pod + Zeroable,
        V: Default + Copy + Clone + Pod + Zeroable,
        const MAX_SIZE: usize,
    > Index<&K> for RedBlackTree<K, V, MAX_SIZE>
{
    type Output = V;

    fn index(&self, index: &K) -> &Self::Output {
        self.get(index).unwrap()
    }
}

impl<
        K: PartialOrd + Copy + Clone + Default + Pod + Zeroable,
        V: Default + Copy + Clone + Pod + Zeroable,
        const MAX_SIZE: usize,
    > IndexMut<&K> for RedBlackTree<K, V, MAX_SIZE>
{
    fn index_mut(&mut self, index: &K) -> &mut Self::Output {
        self.get_mut(index).unwrap()
    }
}

#[test]
/// This test addresses the case where a node's parent and uncle are both red.
/// This is resolved by coloring the parent and uncle black and the grandparent red.
fn test_insert_with_red_parent_and_uncle() {
    type RBT = RedBlackTree<u64, u64, 1024>;
    let mut buf = vec![0u8; std::mem::size_of::<RBT>()];
    let tree = RBT::new_from_slice(buf.as_mut_slice());
    let addrs = vec![
        tree.insert(61, 0).unwrap(),
        tree.insert(52, 0).unwrap(),
        tree.insert(85, 0).unwrap(),
        tree.insert(76, 0).unwrap(),
        tree.insert(93, 0).unwrap(),
    ];

    let parent = addrs[4];
    let uncle = addrs[3];
    let grandparent = addrs[2];

    assert_eq!(tree.get_left(addrs[0]), addrs[1]);
    assert_eq!(tree.get_right(addrs[0]), grandparent);
    assert_eq!(tree.get_parent(addrs[1]), addrs[0]);
    assert_eq!(tree.get_parent(grandparent), addrs[0]);

    assert_eq!(tree.get_left(grandparent), uncle);
    assert_eq!(tree.get_right(grandparent), parent);
    assert_eq!(tree.get_parent(uncle), grandparent);
    assert_eq!(tree.get_parent(parent), grandparent);

    assert!(tree.is_black(addrs[0]) && tree.is_black(addrs[1]) && tree.is_black(grandparent));
    assert!(tree.is_red(uncle) && tree.is_red(parent));

    let leaf = tree.insert(100, 0).unwrap();

    assert!(
        tree.is_black(addrs[0])
            && tree.is_black(addrs[1])
            && tree.is_black(uncle)
            && tree.is_black(parent)
    );
    assert!(tree.is_red(grandparent) && tree.is_red(leaf));
}

#[test]
/// This test addresses the case where a node's parent (P) is red and uncle is black.
/// The new leaf (L) is the right child of the parent and the parent is the right
/// child of the grandparent (G).
///
/// "P is right child of G and L is right child of P."
///
/// We resolve this by rotating the grandparent left and then
/// fixing the colors.
fn test_right_insert_with_red_right_child_parent_and_black_uncle() {
    type RBT = RedBlackTree<u64, u64, 1024>;
    let mut buf = vec![0u8; std::mem::size_of::<RBT>()];
    let tree = RBT::new_from_slice(buf.as_mut_slice());
    let addrs = vec![
        tree.insert(61, 0).unwrap(),
        tree.insert(52, 0).unwrap(),
        tree.insert(85, 0).unwrap(),
        tree.insert(93, 0).unwrap(),
    ];

    let parent = addrs[3];
    // Uncle is black as it is null
    let grandparent = addrs[2];

    assert!(tree.is_black(addrs[0]) && tree.is_black(addrs[1]) && tree.is_black(grandparent));
    assert!(tree.is_red(parent));

    assert_eq!(tree.get_left(addrs[0]), addrs[1]);
    assert_eq!(tree.get_right(addrs[0]), grandparent);
    assert_eq!(tree.get_parent(addrs[1]), addrs[0]);
    assert_eq!(tree.get_parent(grandparent), addrs[0]);

    assert_eq!(tree.get_left(grandparent), SENTINEL);
    assert_eq!(tree.get_right(grandparent), parent);
    assert_eq!(tree.get_parent(parent), grandparent);

    let leaf = tree.insert(100, 0).unwrap();

    assert!(tree.is_black(addrs[0]) && tree.is_black(addrs[1]) && tree.is_black(parent));
    assert!(tree.is_red(grandparent) && tree.is_red(leaf));

    assert_eq!(tree.get_left(addrs[0]), addrs[1]);
    assert_eq!(tree.get_right(addrs[0]), parent);
    assert_eq!(tree.get_parent(addrs[1]), addrs[0]);
    assert_eq!(tree.get_parent(parent), addrs[0]);

    assert_eq!(tree.get_left(parent), grandparent);
    assert_eq!(tree.get_right(parent), leaf);
    assert_eq!(tree.get_parent(grandparent), parent);
    assert_eq!(tree.get_parent(leaf), parent);
    assert!(tree.is_leaf(leaf) && tree.is_leaf(grandparent));
}

#[test]
/// This test addresses the case where a node's parent is red and uncle is black.
/// The new leaf is the left child of the parent and the parent is the right
/// child of the grandparent.
///
/// "P is right child of G and L is left child of P."
///
/// We resolve this by rotating the parent right then applying the same
/// algorithm as the previous test.
fn test_left_insert_with_red_right_child_parent_and_black_uncle() {
    type RBT = RedBlackTree<u64, u64, 1024>;
    let mut buf = vec![0u8; std::mem::size_of::<RBT>()];
    let tree = RBT::new_from_slice(buf.as_mut_slice());
    let addrs = vec![
        tree.insert(61, 0).unwrap(),
        tree.insert(52, 0).unwrap(),
        tree.insert(85, 0).unwrap(),
        tree.insert(93, 0).unwrap(),
    ];

    let parent = addrs[3];
    // Uncle is black as it is null
    let grandparent = addrs[2];

    assert!(tree.is_black(addrs[0]) && tree.is_black(addrs[1]) && tree.is_black(grandparent));
    assert!(tree.is_red(parent));

    assert_eq!(tree.get_left(addrs[0]), addrs[1]);
    assert_eq!(tree.get_right(addrs[0]), grandparent);
    assert_eq!(tree.get_parent(addrs[1]), addrs[0]);
    assert_eq!(tree.get_parent(grandparent), addrs[0]);

    assert_eq!(tree.get_left(grandparent), SENTINEL);
    assert_eq!(tree.get_right(grandparent), parent);
    assert_eq!(tree.get_parent(parent), grandparent);

    let leaf = tree.insert(87, 0).unwrap();

    assert!(tree.is_black(addrs[0]) && tree.is_black(addrs[1]) && tree.is_black(leaf));
    assert!(tree.is_red(grandparent) && tree.is_red(parent));

    assert_eq!(tree.get_left(addrs[0]), addrs[1]);
    assert_eq!(tree.get_right(addrs[0]), leaf);
    assert_eq!(tree.get_parent(addrs[1]), addrs[0]);
    assert_eq!(tree.get_parent(leaf), addrs[0]);

    assert_eq!(tree.get_left(leaf), grandparent);
    assert_eq!(tree.get_right(leaf), parent);
    assert_eq!(tree.get_parent(grandparent), leaf);
    assert_eq!(tree.get_parent(parent), leaf);
    assert!(tree.is_leaf(parent) && tree.is_leaf(grandparent));
}

#[test]
/// This test addresses the case where a node's parent is red and uncle is black.
/// The new leaf is the left child of the parent and the parent is the left
/// child of the grandparent.
///
/// "P is left child of G and L is left child of P."
///
/// We resolve this by rotating the grandparent right and then
/// fixing the colors.
fn test_left_insert_with_red_left_child_parent_and_black_uncle() {
    type RBT = RedBlackTree<u64, u64, 1024>;
    let mut buf = vec![0u8; std::mem::size_of::<RBT>()];
    let tree = RBT::new_from_slice(buf.as_mut_slice());
    let addrs = vec![
        tree.insert(61, 0).unwrap(),
        tree.insert(85, 0).unwrap(),
        tree.insert(52, 0).unwrap(),
        tree.insert(41, 0).unwrap(),
    ];

    let parent = addrs[3];
    // Uncle is black as it is null
    let grandparent = addrs[2];

    assert!(tree.is_black(addrs[0]) && tree.is_black(addrs[1]) && tree.is_black(grandparent));
    assert!(tree.is_red(parent));

    assert_eq!(tree.get_right(addrs[0]), addrs[1]);
    assert_eq!(tree.get_left(addrs[0]), grandparent);
    assert_eq!(tree.get_parent(addrs[1]), addrs[0]);
    assert_eq!(tree.get_parent(grandparent), addrs[0]);

    assert_eq!(tree.get_right(grandparent), SENTINEL);
    assert_eq!(tree.get_left(grandparent), parent);
    assert_eq!(tree.get_parent(parent), grandparent);

    let leaf = tree.insert(25, 0).unwrap();

    assert!(tree.is_black(addrs[0]) && tree.is_black(addrs[1]) && tree.is_black(parent));
    assert!(tree.is_red(grandparent) && tree.is_red(leaf));

    assert_eq!(tree.get_right(addrs[0]), addrs[1]);
    assert_eq!(tree.get_left(addrs[0]), parent);
    assert_eq!(tree.get_parent(addrs[1]), addrs[0]);
    assert_eq!(tree.get_parent(parent), addrs[0]);

    assert_eq!(tree.get_right(parent), grandparent);
    assert_eq!(tree.get_left(parent), leaf);
    assert_eq!(tree.get_parent(grandparent), parent);
    assert_eq!(tree.get_parent(leaf), parent);
    assert!(tree.is_leaf(leaf) && tree.is_leaf(grandparent));
}

#[test]
/// This test addresses the case where a node's parent is red and uncle is black.
/// The new leaf is the right child of the parent and the parent is the left
/// child of the grandparent.
///
/// "P is left child of G and L is right child of P."
///
/// We resolve this by rotating the parent left then applying the same
/// algorithm as the previous test.
fn test_right_insert_with_red_left_child_parent_and_black_uncle() {
    type RBT = RedBlackTree<u64, u64, 1024>;
    let mut buf = vec![0u8; std::mem::size_of::<RBT>()];
    let tree = RBT::new_from_slice(buf.as_mut_slice());
    let addrs = vec![
        tree.insert(61, 0).unwrap(),
        tree.insert(85, 0).unwrap(),
        tree.insert(52, 0).unwrap(),
        tree.insert(41, 0).unwrap(),
    ];

    let parent = addrs[3];
    // Uncle is black as it is null
    let grandparent = addrs[2];

    assert!(tree.is_black(addrs[0]) && tree.is_black(addrs[1]) && tree.is_black(grandparent));
    assert!(tree.is_red(parent));

    assert_eq!(tree.get_right(addrs[0]), addrs[1]);
    assert_eq!(tree.get_left(addrs[0]), grandparent);
    assert_eq!(tree.get_parent(addrs[1]), addrs[0]);
    assert_eq!(tree.get_parent(grandparent), addrs[0]);

    assert_eq!(tree.get_right(grandparent), SENTINEL);
    assert_eq!(tree.get_left(grandparent), parent);
    assert_eq!(tree.get_parent(parent), grandparent);

    let leaf = tree.insert(47, 0).unwrap();

    assert!(tree.is_black(addrs[0]) && tree.is_black(addrs[1]) && tree.is_black(leaf));
    assert!(tree.is_red(grandparent) && tree.is_red(parent));

    assert_eq!(tree.get_right(addrs[0]), addrs[1]);
    assert_eq!(tree.get_left(addrs[0]), leaf);
    assert_eq!(tree.get_parent(addrs[1]), addrs[0]);
    assert_eq!(tree.get_parent(leaf), addrs[0]);

    assert_eq!(tree.get_right(leaf), grandparent);
    assert_eq!(tree.get_left(leaf), parent);
    assert_eq!(tree.get_parent(grandparent), leaf);
    assert_eq!(tree.get_parent(parent), leaf);
    assert!(tree.is_leaf(parent) && tree.is_leaf(grandparent));
}
