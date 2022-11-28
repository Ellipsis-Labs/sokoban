use bytemuck::{Pod, Zeroable};
use colored::Colorize;
use num_derive::FromPrimitive;
use num_traits::FromPrimitive;
use std::{
    collections::BTreeMap,
    fmt::{Debug, Display},
    ops::{Index, IndexMut},
};

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
    K: PartialOrd + Ord + Copy + Clone + Default + Pod + Zeroable,
    V: Default + Copy + Clone + Pod + Zeroable,
> {
    pub key: K,
    pub value: V,
}

unsafe impl<
        K: PartialOrd + Ord + Copy + Clone + Default + Pod + Zeroable,
        V: Default + Copy + Clone + Pod + Zeroable,
    > Zeroable for RBNode<K, V>
{
}
unsafe impl<
        K: PartialOrd + Ord + Copy + Clone + Default + Pod + Zeroable,
        V: Default + Copy + Clone + Pod + Zeroable,
    > Pod for RBNode<K, V>
{
}

impl<
        K: PartialOrd + Ord + Copy + Clone + Default + Pod + Zeroable,
        V: Default + Copy + Clone + Pod + Zeroable,
    > RBNode<K, V>
{
    pub fn new(key: K, value: V) -> Self {
        Self { key, value }
    }
}

#[derive(Copy, Clone)]
pub struct RedBlackTree<
    K: PartialOrd + Ord + Copy + Clone + Default + Pod + Zeroable,
    V: Default + Copy + Clone + Pod + Zeroable,
    const MAX_SIZE: usize,
> {
    pub root: u64,
    _padding: u64,
    allocator: NodeAllocator<RBNode<K, V>, MAX_SIZE, 4>,
}

unsafe impl<
        K: PartialOrd + Ord + Copy + Clone + Default + Pod + Zeroable,
        V: Default + Copy + Clone + Pod + Zeroable,
        const MAX_SIZE: usize,
    > Zeroable for RedBlackTree<K, V, MAX_SIZE>
{
}
unsafe impl<
        K: PartialOrd + Ord + Copy + Clone + Default + Pod + Zeroable,
        V: Default + Copy + Clone + Pod + Zeroable,
        const MAX_SIZE: usize,
    > Pod for RedBlackTree<K, V, MAX_SIZE>
{
}

impl<
        K: PartialOrd + Ord + Copy + Clone + Default + Pod + Zeroable,
        V: Default + Copy + Clone + Pod + Zeroable,
        const MAX_SIZE: usize,
    > ZeroCopy for RedBlackTree<K, V, MAX_SIZE>
{
}

impl<
        K: PartialOrd + Ord + Copy + Clone + Default + Pod + Zeroable,
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
        K: PartialOrd + Ord + Copy + Clone + Default + Pod + Zeroable,
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
        K: PartialOrd + Ord + Copy + Clone + Default + Pod + Zeroable,
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
        K: PartialOrd + Ord + Copy + Clone + Default + Pod + Zeroable,
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
        K: PartialOrd + Ord + Copy + Clone + Default + Pod + Zeroable,
        V: Default + Copy + Clone + Pod + Zeroable,
        const MAX_SIZE: usize,
    > RedBlackTree<K, V, MAX_SIZE>
{
    pub fn pretty_print(&self)
    where
        K: Debug + Display,
    {
        let mut s = String::new();
        let mut stack = vec![(self.root as u32, "".to_string(), "".to_string())];
        let key_to_index = self
            .iter()
            .enumerate()
            .map(|(i, (k, _))| (k, i))
            .collect::<BTreeMap<_, _>>();

        while !stack.is_empty() {
            let (node, mut padding, pointer) = stack.pop().unwrap();
            if node == SENTINEL {
                continue;
            }
            let key = self.get_node(node).key;
            s.push_str(&padding);
            s.push_str(&pointer);
            if self.is_red(node) {
                s.push_str(&format!(
                    "\u{001b}[31m{:?} ({})\u{001b}[0m",
                    key, key_to_index[&key]
                ));
            } else {
                s.push_str(&format!("{:?} ({})", key, key_to_index[&key]));
            }
            s.push_str("\n");
            padding.push_str("│  ");

            let right_pointer = "└──".to_string();
            let left_pointer = if self.get_right(node) != SENTINEL {
                "├──".to_string()
            } else {
                "└──".to_string()
            };

            stack.push((self.get_right(node), padding.clone(), right_pointer));
            stack.push((self.get_left(node), padding.clone(), left_pointer));
        }
        println!("{}", s);
    }

    fn assert_proper_alignment() {
        // TODO is this a sufficient coverage of the edge cases?
        assert!(std::mem::size_of::<V>() % std::mem::align_of::<K>() == 0);
        assert!(std::mem::size_of::<RBNode<K, V>>() % std::mem::align_of::<RBNode<K, V>>() == 0);
        assert!(std::mem::size_of::<RBNode<K, V>>() % 8_usize == 0);
    }

    pub fn is_valid_red_black_tree(&self) -> bool {
        // The root is black
        if self.is_red(self.root as u32) {
            return false;
        }

        let mut stack = vec![(self.root as u32, 0)];
        let mut black_count = vec![];

        while !stack.is_empty() {
            let (node_index, mut count) = stack.pop().unwrap();
            count += self.is_black(node_index) as u32;
            if self.is_leaf(node_index) {
                black_count.push(count);
                continue;
            }
            for child in [self.get_left(node_index), self.get_right(node_index)] {
                if child == SENTINEL {
                    continue;
                }
                // Red nodes cannot have red children
                if self.is_red(node_index) && self.is_red(child) {
                    return false;
                }
                stack.push((child, count));
            }
        }
        // All paths from root to leaf must have the same number of black nodes
        let branch_len_equal = black_count.iter().all(|&x| x == black_count[0]);
        if !branch_len_equal {
            println!("Branch lengths not equal: {:?}", black_count);
            return false;
        }
        true
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
    fn color_node(&mut self, node: u32, color: u32) {
        self.allocator.set_register(node, color, COLOR);
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

    pub fn get_dir(&self, node: u32, dir: u32) -> u32 {
        if dir == Field::Left as u32 {
            self.get_left(node)
        } else {
            self.get_right(node)
        }
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

    fn remove_node(&mut self, node: u32) {
        // Clear all registers
        self.allocator.clear_register(node, Field::Parent as u32);
        self.allocator.clear_register(node, COLOR);
        self.allocator.clear_register(node, Field::Left as u32);
        self.allocator.clear_register(node, Field::Right as u32);
        // Add free slot to the free list
        self.allocator.remove_node(node);
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
        if !matches!(
            FromPrimitive::from_u32(dir),
            Some(Field::Left) | Some(Field::Right),
        ) {
            return None;
        }
        let sibling_index = self.get_child(parent_index, opposite(dir));
        if sibling_index == SENTINEL {
            return None;
        }
        let child_index = self.get_child(sibling_index, dir);
        self.connect(sibling_index, parent_index, dir);
        self.connect(parent_index, child_index, opposite(dir));
        if grandparent_index != SENTINEL {
            self.connect(
                grandparent_index,
                sibling_index,
                self.child_dir(grandparent_index, parent_index),
            );
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

    fn fix_remove(&mut self, mut node_index: u32, parent_dir: Option<(u32, u32)>) {
        if node_index == SENTINEL {
            return;
        }
        while node_index != self.root as u32 && self.is_black(node_index) {
            let (parent, dir) = if node_index == SENTINEL {
                parent_dir.unwrap()
            } else {
                let parent = self.get_parent(node_index);
                let dir = self.child_dir(parent, node_index);
                (parent, dir)
            };
            let mut sibling = self.get_child(parent, opposite(dir));
            if self.is_red(sibling) {
                self.color_black(sibling);
                self.color_red(parent);
                self.rotate_dir(parent, dir);
                sibling = self.get_dir(parent, opposite(dir));
            }
            if self.is_black(self.get_left(sibling)) && self.is_black(self.get_right(sibling)) {
                self.color_red(sibling);
                node_index = parent;
            } else {
                if self.is_black(self.get_dir(sibling, opposite(dir))) {
                    self.color_black(self.get_dir(sibling, dir));
                    self.color_red(sibling);
                    self.rotate_dir(sibling, opposite(dir));
                    sibling = self.get_dir(parent, opposite(dir));
                }
                self.color_node(sibling, self.get_color(parent));
                self.color_black(parent);
                self.color_black(self.get_dir(sibling, opposite(dir)));
                self.rotate_dir(parent, dir);
                node_index = self.root as u32;
            }
        }
        self.color_black(node_index);
    }

    fn _remove(&mut self, key: &K) -> Option<V> {
        let mut curr_node_index = self.root as u32;
        if curr_node_index == SENTINEL {
            return None;
        }
        loop {
            let curr_key = self.allocator.get(curr_node_index).get_value().key;
            let curr_value = self.allocator.get(curr_node_index).get_value().value;
            let left = self.get_left(curr_node_index);
            let right = self.get_right(curr_node_index);
            let target = if *key < curr_key {
                left
            } else if *key > curr_key {
                right
            } else {
                // We have found the node to remove
                let mut is_black = self.is_black(curr_node_index);
                let (pivot_node_index, parent_dir) = if left == SENTINEL {
                    self.transplant(curr_node_index, right);
                    (right, None)
                } else if right == SENTINEL {
                    self.transplant(curr_node_index, left);
                    (left, None)
                } else {
                    // Find the largest node in the left subtree
                    let mut parent_dir = None;
                    let max_left = self.find_max(left);
                    let max_left_child = self.get_left(max_left);
                    is_black = self.is_black(max_left);
                    // If max_left is not equal to root of the left subtree, then
                    // replace the root of the left subtree with max_left and replace
                    // max_left with max_left's child
                    if self.get_parent(max_left) != curr_node_index {
                        self.transplant(max_left, max_left_child);
                        self.connect(
                            max_left,
                            self.get_left(curr_node_index),
                            Field::Right as u32,
                        );
                        if max_left_child == SENTINEL {
                            parent_dir = Some((self.get_parent(max_left), Field::Right as u32));
                        }
                    } else {
                        if max_left_child == SENTINEL {
                            parent_dir = Some((max_left, Field::Left as u32));
                        }
                    }
                    // Complete the transplant of max_left
                    self.transplant(curr_node_index, max_left);
                    self.connect(
                        max_left,
                        self.get_right(curr_node_index),
                        Field::Right as u32,
                    );
                    self.color_node(max_left, self.get_color(curr_node_index));

                    (max_left_child, parent_dir)
                };

                // Completely remove the current node index from the tree
                self.remove_node(curr_node_index);

                if is_black {
                    self.fix_remove(pivot_node_index, parent_dir);
                }
                return Some(curr_value);
            };
            if target == SENTINEL {
                return None;
            }
            curr_node_index = target
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
        K: PartialOrd + Ord + Copy + Clone + Default + Pod + Zeroable,
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
        K: PartialOrd + Ord + Copy + Clone + Default + Pod + Zeroable,
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
    K: PartialOrd + Ord + Copy + Clone + Default + Pod + Zeroable,
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
        K: PartialOrd + Ord + Copy + Clone + Default + Pod + Zeroable,
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
        K: PartialOrd + Ord + Copy + Clone + Default + Pod + Zeroable,
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
    K: PartialOrd + Ord + Copy + Clone + Default + Pod + Zeroable,
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
        K: PartialOrd + Ord + Copy + Clone + Default + Pod + Zeroable,
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
        K: PartialOrd + Ord + Copy + Clone + Default + Pod + Zeroable,
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
        K: PartialOrd + Ord + Copy + Clone + Default + Pod + Zeroable,
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
        K: PartialOrd + Ord + Copy + Clone + Default + Pod + Zeroable,
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

#[test]
fn test_delete_multiple_random() {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    type RBT = RedBlackTree<u64, u64, 32>;
    let mut buf = vec![0u8; std::mem::size_of::<RBT>()];
    let sample_tree = RBT::new_from_slice(buf.as_mut_slice());
    let mut dummy_keys = vec![];
    // Fill up tree
    for k in 0..32 {
        let mut hasher = DefaultHasher::new();
        (k as u64).hash(&mut hasher);
        let key = hasher.finish();
        sample_tree.insert(key, 0).unwrap();
        assert!(sample_tree.is_valid_red_black_tree());
        dummy_keys.push(key);
    }

    let mut keys = vec![];
    let mut buf = vec![0u8; std::mem::size_of::<RBT>()];
    let index_tree = RBT::new_from_slice(buf.as_mut_slice());
    let key_to_index = sample_tree
        .iter()
        .enumerate()
        .map(|(i, (k, _))| (*k, i as u64))
        .collect::<BTreeMap<_, _>>();

    for k in dummy_keys.iter() {
        let i = key_to_index[k];
        println!("Inserting {} -> {}", k, i);
        index_tree.insert(i, 0).unwrap();
        keys.push(i);
        assert!(index_tree.is_valid_red_black_tree());
    }

    sample_tree.pretty_print();
    index_tree.pretty_print();

    for i in keys.iter() {
        println!("Removing {}", i);
        index_tree.remove(&i).unwrap();
        index_tree.pretty_print();
        assert!(index_tree.is_valid_red_black_tree());
        if *i == 6 {
            panic!("Stop here");
        }
    }
}
