use bytemuck::{Pod, Zeroable};
use num_derive::FromPrimitive;
use num_traits::FromPrimitive;
use std::{
    cmp::Ordering,
    fmt::Debug,
    ops::{Index, IndexMut},
    vec,
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

#[repr(C)]
#[derive(Copy, Clone)]
pub struct RedBlackTree<
    K: PartialOrd + Ord + Copy + Clone + Default + Pod + Zeroable,
    V: Default + Copy + Clone + Pod + Zeroable,
    const MAX_SIZE: usize,
> {
    pub root: u32,
    _padding: [u32; 3],
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
        K: Debug + PartialOrd + Ord + Copy + Clone + Default + Pod + Zeroable,
        V: Default + Copy + Clone + Pod + Zeroable,
        const MAX_SIZE: usize,
    > Default for RedBlackTree<K, V, MAX_SIZE>
{
    fn default() -> Self {
        Self::assert_proper_alignment();
        RedBlackTree {
            root: SENTINEL,
            _padding: [0; 3],
            allocator: NodeAllocator::<RBNode<K, V>, MAX_SIZE, 4>::default(),
        }
    }
}

impl<
        K: Debug + PartialOrd + Ord + Copy + Clone + Default + Pod + Zeroable,
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
        K: Debug + PartialOrd + Ord + Copy + Clone + Default + Pod + Zeroable,
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
        let node_index = self.get_addr(key);
        if node_index == SENTINEL {
            None
        } else {
            Some(&self.get_node(node_index).value)
        }
    }

    fn get_mut(&mut self, key: &K) -> Option<&mut V> {
        let node_index = self.get_addr(key);
        if node_index == SENTINEL {
            None
        } else {
            Some(&mut self.get_node_mut(node_index).value)
        }
    }

    fn size(&self) -> usize {
        self.allocator.size as usize
    }

    fn len(&self) -> usize {
        self.allocator.size as usize
    }

    fn capacity(&self) -> usize {
        MAX_SIZE
    }

    fn iter(&self) -> Box<dyn DoubleEndedIterator<Item = (&K, &V)> + '_> {
        Box::new(self._iter())
    }

    fn iter_mut(&mut self) -> Box<dyn DoubleEndedIterator<Item = (&K, &mut V)> + '_> {
        Box::new(self._iter_mut())
    }
}

impl<
        K: Debug + PartialOrd + Ord + Copy + Clone + Default + Pod + Zeroable,
        V: Default + Copy + Clone + Pod + Zeroable,
        const MAX_SIZE: usize,
    > OrderedNodeAllocatorMap<K, V> for RedBlackTree<K, V, MAX_SIZE>
{
    fn get_min_index(&mut self) -> u32 {
        self._find_min(self.root)
    }

    fn get_max_index(&mut self) -> u32 {
        self._find_max(self.root)
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
        K: Debug + PartialOrd + Ord + Copy + Clone + Default + Pod + Zeroable,
        V: Default + Copy + Clone + Pod + Zeroable,
        const MAX_SIZE: usize,
    > RedBlackTree<K, V, MAX_SIZE>
{
    pub fn pretty_print(&self) {
        if self.len() == 0 {
            return;
        }
        let mut s = String::new();
        let mut stack = vec![(self.root, "".to_string(), "".to_string())];

        while !stack.is_empty() {
            let (node, mut padding, pointer) = stack.pop().unwrap();
            if node == SENTINEL {
                continue;
            }
            let key = self.get_node(node).key;
            s.push_str(&padding);
            s.push_str(&pointer);
            if self.is_red(node) {
                // Prints red nodes in red
                s.push_str(&format!("\u{001b}[31m{:?}\u{001b}[0m", key));
            } else {
                s.push_str(&format!("{:?}", key));
            }
            s.push('\n');
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
        if self.len() == 0 {
            return true;
        }
        // The root must be black
        if self.is_red(self.root) {
            println!("Invalid Red-Black Tree: Root is red");
            return false;
        }

        let mut stack = vec![(self.root, 0)];
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
                    println!(
                        "Invalid Red-Black Tree: Red node (key: {:?}) has red child",
                        self.get_node(node_index).key
                    );
                    return false;
                }
                stack.push((child, count));
            }
        }
        // All paths from root to leaf must have the same number of black nodes
        let balanced = black_count.iter().all(|&x| x == black_count[0]);
        if !balanced {
            println!("Invalid Red-Black Tree: All paths must have the same number of black nodes",);
        }
        balanced
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
    fn _color_red(&mut self, node: u32) {
        if node != SENTINEL {
            self.allocator.set_register(node, Color::Red as u32, COLOR);
        }
    }

    #[inline(always)]
    fn _color_black(&mut self, node: u32) {
        self.allocator
            .set_register(node, Color::Black as u32, COLOR);
    }

    #[inline(always)]
    fn _color_node(&mut self, node: u32, color: u32) {
        self.allocator.set_register(node, color, COLOR);
    }

    #[inline(always)]
    pub fn is_red(&self, node: u32) -> bool {
        self.allocator.get_register(node, COLOR) == Color::Red as u32
    }

    #[inline(always)]
    pub fn is_black(&self, node: u32) -> bool {
        self.allocator.get_register(node, COLOR) == Color::Black as u32
    }

    #[inline(always)]
    pub fn get_child(&self, node: u32, dir: u32) -> u32 {
        self.allocator.get_register(node, dir)
    }

    #[inline(always)]
    pub fn is_leaf(&self, node: u32) -> bool {
        self.get_left(node) == SENTINEL && self.get_right(node) == SENTINEL
    }

    #[inline(always)]
    pub fn is_root(&self, node: u32) -> bool {
        self.root == node
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

    fn _remove_allocator_node(&mut self, node: u32) {
        // Clear all registers
        self.allocator.clear_register(node, Field::Parent as u32);
        self.allocator.clear_register(node, COLOR);
        self.allocator.clear_register(node, Field::Left as u32);
        self.allocator.clear_register(node, Field::Right as u32);
        // Add free slot to the free list
        self.allocator.remove_node(node);
    }

    #[inline(always)]
    fn _connect(&mut self, parent: u32, child: u32, dir: u32) {
        self.allocator
            .connect(parent, child, dir, Field::Parent as u32);
    }

    #[inline(always)]
    fn _child_dir(&self, parent: u32, child: u32) -> u32 {
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

    fn _rotate_dir(&mut self, parent_index: u32, dir: u32) -> Option<u32> {
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
        self._connect(sibling_index, parent_index, dir);
        self._connect(parent_index, child_index, opposite(dir));
        if grandparent_index != SENTINEL {
            self._connect(
                grandparent_index,
                sibling_index,
                self._child_dir(grandparent_index, parent_index),
            );
        } else {
            self.allocator
                .clear_register(sibling_index, Field::Parent as u32);
            self.root = sibling_index;
        }
        Some(sibling_index)
    }

    fn _insert(&mut self, key: K, value: V) -> Option<u32> {
        let mut parent_node_index = self.root;
        let new_node = RBNode::<K, V>::new(key, value);
        if parent_node_index == SENTINEL {
            let node_index = self.allocator.add_node(new_node);
            self.root = node_index;
            return Some(node_index);
        }
        loop {
            let curr_key = self.get_node(parent_node_index).key;
            let (target, dir) = match key.cmp(&curr_key) {
                Ordering::Less => (self.get_left(parent_node_index), Field::Left as u32),
                Ordering::Greater => (self.get_right(parent_node_index), Field::Right as u32),
                Ordering::Equal => {
                    self.get_node_mut(parent_node_index).value = value;
                    return Some(parent_node_index);
                }
            };
            if target == SENTINEL {
                if self.len() >= self.capacity() {
                    return None;
                }
                let node_index = self.allocator.add_node(new_node);
                self._color_red(node_index);
                self._connect(parent_node_index, node_index, dir);
                let grandparent = self.get_parent(parent_node_index);
                // This is only false when the parent is the root
                if grandparent != SENTINEL {
                    self._fix_insert(node_index);
                }
                return Some(node_index);
            }
            parent_node_index = target
        }
    }

    fn _fix_insert(&mut self, mut node: u32) -> Option<()> {
        while self.is_red(self.get_parent(node)) {
            let mut parent = self.get_parent(node);
            let mut grandparent = self.get_parent(parent);
            if grandparent == SENTINEL {
                assert!(self.is_root(parent));
                break;
            }
            let dir = self._child_dir(grandparent, parent);
            let uncle = self.get_child(grandparent, opposite(dir));
            if self.is_red(uncle) {
                self._color_black(uncle);
                self._color_black(parent);
                self._color_red(grandparent);
                node = grandparent;
            } else {
                if self._child_dir(parent, node) == opposite(dir) {
                    self._rotate_dir(parent, dir);
                    node = parent;
                }
                parent = self.get_parent(node);
                grandparent = self.get_parent(parent);
                self._color_black(parent);
                self._color_red(grandparent);
                self._rotate_dir(grandparent, opposite(dir));
            }
        }
        self._color_black(self.root as u32);
        Some(())
    }

    fn _remove(&mut self, key: &K) -> Option<V> {
        let mut curr_node_index = self.root as u32;
        if curr_node_index == SENTINEL {
            return None;
        }
        loop {
            let RBNode {
                key: curr_key,
                value: curr_value,
            } = *self.allocator.get(curr_node_index).get_value();
            let target = match key.cmp(&curr_key) {
                Ordering::Less => self.get_left(curr_node_index),
                Ordering::Greater => self.get_right(curr_node_index),
                Ordering::Equal => {
                    self._remove_tree_node(curr_node_index);
                    return Some(curr_value);
                }
            };
            if target == SENTINEL {
                return None;
            }
            curr_node_index = target
        }
    }

    fn _remove_tree_node(&mut self, node_index: u32) {
        let mut is_black = self.is_black(node_index);
        let left = self.get_left(node_index);
        let right = self.get_right(node_index);
        let (pivot_node_index, parent_and_dir) = if self.is_leaf(node_index) {
            if !self.is_root(node_index) {
                let parent = self.get_parent(node_index);
                let dir = self._child_dir(parent, node_index);
                // Remove pointer to the removed leaf node
                self._connect(parent, SENTINEL, dir);
                (SENTINEL, Some((parent, dir)))
            } else {
                // Set the root to SENTINEL
                self.root = SENTINEL;
                (SENTINEL, None)
            }
        } else if left == SENTINEL {
            self._transplant(node_index, right);
            (right, None)
        } else if right == SENTINEL {
            self._transplant(node_index, left);
            (left, None)
        } else {
            // Find the largest node in the left subtree
            let mut parent_and_dir = None;
            let max_left = self._find_max(left);
            let max_left_parent = self.get_parent(max_left);
            let max_left_child = self.get_left(max_left);
            is_black = self.is_black(max_left);

            // If max_left is not equal to root of the left subtree, then
            // replace the root of the left subtree with max_left and replace
            // max_left with max_left_child
            if self.get_parent(max_left) != node_index {
                self._transplant(max_left, max_left_child);
                // We perform this operation in the conditional because we do not
                // want to form a cycle
                self._connect(max_left, self.get_left(node_index), Field::Left as u32);
                if max_left_child == SENTINEL {
                    parent_and_dir = Some((max_left_parent, Field::Right as u32));
                }
            } else if max_left_child == SENTINEL {
                // The only time this is called is when the left subtree is
                // a single node
                assert!(self.is_leaf(max_left));
                parent_and_dir = Some((max_left, Field::Left as u32));
            }

            // Complete the transplant of max_left
            self._transplant(node_index, max_left);
            self._connect(max_left, self.get_right(node_index), Field::Right as u32);

            self._color_node(max_left, self.get_color(node_index));

            (max_left_child, parent_and_dir)
        };

        // Completely remove the current node index from the tree
        self._remove_allocator_node(node_index);

        if is_black {
            if self.is_root(pivot_node_index) {
                self._color_black(pivot_node_index);
            } else {
                self._fix_remove(pivot_node_index, parent_and_dir);
            }
        }
    }

    fn _fix_remove(&mut self, mut node_index: u32, parent_and_dir: Option<(u32, u32)>) {
        let (mut parent, mut dir) = parent_and_dir.unwrap_or({
            let parent = self.get_parent(node_index);
            let dir = self._child_dir(parent, node_index);
            (parent, dir)
        });
        loop {
            let mut sibling = self.get_child(parent, opposite(dir));
            if self.is_red(sibling) {
                self._color_black(sibling);
                self._color_red(parent);
                self._rotate_dir(parent, dir);
                sibling = self.get_dir(parent, opposite(dir));
            }
            if self.is_black(self.get_left(sibling)) && self.is_black(self.get_right(sibling)) {
                self._color_red(sibling);
                node_index = parent;
            } else {
                if self.is_black(self.get_dir(sibling, opposite(dir))) {
                    self._color_black(self.get_dir(sibling, dir));
                    self._color_red(sibling);
                    self._rotate_dir(sibling, opposite(dir));
                    sibling = self.get_dir(parent, opposite(dir));
                }
                self._color_node(sibling, self.get_color(parent));
                self._color_black(parent);
                self._color_black(self.get_dir(sibling, opposite(dir)));
                self._rotate_dir(parent, dir);
                node_index = self.root as u32;
            }
            if self.is_root(node_index) || self.is_red(node_index) {
                break;
            }
            parent = self.get_parent(node_index);
            dir = self._child_dir(parent, node_index);
        }
        self._color_black(node_index);
    }

    #[inline(always)]
    /// This helper function connects the parent of `target` to `source`.
    /// It is the start of the process of removing `target` from the tree.
    fn _transplant(&mut self, target: u32, source: u32) {
        let parent = self.get_parent(target);
        if parent == SENTINEL {
            self.root = source;
            self.allocator
                .set_register(source, SENTINEL, Field::Parent as u32);
            return;
        }
        let dir = self._child_dir(parent, target);
        self._connect(parent, source, dir);
    }

    pub fn get_addr(&self, key: &K) -> u32 {
        let mut node_index = self.root;
        if node_index == SENTINEL {
            return SENTINEL;
        }
        loop {
            let curr_key = self.get_node(node_index).key;
            let target = match key.cmp(&curr_key) {
                Ordering::Less => self.get_left(node_index),
                Ordering::Greater => self.get_right(node_index),
                Ordering::Equal => return node_index,
            };
            if target == SENTINEL {
                return SENTINEL;
            }
            node_index = target
        }
    }

    fn _find_min(&self, index: u32) -> u32 {
        let mut node = index;
        while self.get_left(node) != SENTINEL {
            node = self.get_left(node);
        }
        node
    }

    fn _find_max(&self, index: u32) -> u32 {
        let mut node = index;
        while self.get_right(node) != SENTINEL {
            node = self.get_right(node);
        }
        node
    }

    fn _iter(&self) -> RedBlackTreeIterator<'_, K, V, MAX_SIZE> {
        RedBlackTreeIterator::<K, V, MAX_SIZE> {
            tree: self,
            fwd_stack: vec![],
            fwd_ptr: self.root,
            fwd_node: None,
            rev_stack: vec![],
            rev_ptr: self.root,
            rev_node: None,
            terminated: false,
        }
    }

    fn _iter_mut(&mut self) -> RedBlackTreeIteratorMut<'_, K, V, MAX_SIZE> {
        let node = self.root;
        RedBlackTreeIteratorMut::<K, V, MAX_SIZE> {
            tree: self,
            fwd_stack: vec![],
            fwd_ptr: node,
            fwd_node: None,
            rev_stack: vec![],
            rev_ptr: node,
            rev_node: None,
            terminated: false,
        }
    }
}

impl<
        'a,
        K: Debug + PartialOrd + Ord + Copy + Clone + Default + Pod + Zeroable,
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
        K: Debug + PartialOrd + Ord + Copy + Clone + Default + Pod + Zeroable,
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
    K: Debug + PartialOrd + Ord + Copy + Clone + Default + Pod + Zeroable,
    V: Default + Copy + Clone + Pod + Zeroable,
    const MAX_SIZE: usize,
> {
    tree: &'a RedBlackTree<K, V, MAX_SIZE>,
    fwd_stack: Vec<u32>,
    fwd_ptr: u32,
    fwd_node: Option<u32>,
    rev_stack: Vec<u32>,
    rev_ptr: u32,
    rev_node: Option<u32>,
    terminated: bool,
}

impl<
        'a,
        K: Debug + PartialOrd + Ord + Copy + Clone + Default + Pod + Zeroable,
        V: Default + Copy + Clone + Pod + Zeroable,
        const MAX_SIZE: usize,
    > Iterator for RedBlackTreeIterator<'a, K, V, MAX_SIZE>
{
    type Item = (&'a K, &'a V);

    fn next(&mut self) -> Option<Self::Item> {
        while !self.terminated && (!self.fwd_stack.is_empty() || self.fwd_ptr != SENTINEL) {
            if self.fwd_ptr != SENTINEL {
                self.fwd_stack.push(self.fwd_ptr);
                self.fwd_ptr = self.tree.get_left(self.fwd_ptr);
            } else {
                let current_node = self.fwd_stack.pop();
                if current_node == self.rev_node {
                    self.terminated = true;
                    return None;
                }
                self.fwd_node = current_node;
                let node = self.tree.get_node(current_node.unwrap());
                self.fwd_ptr = self.tree.get_right(current_node.unwrap());
                return Some((&node.key, &node.value));
            }
        }
        None
    }
}

impl<
        'a,
        K: Debug + PartialOrd + Ord + Copy + Clone + Default + Pod + Zeroable,
        V: Default + Copy + Clone + Pod + Zeroable,
        const MAX_SIZE: usize,
    > DoubleEndedIterator for RedBlackTreeIterator<'a, K, V, MAX_SIZE>
{
    fn next_back(&mut self) -> Option<Self::Item> {
        while !self.terminated && (!self.rev_stack.is_empty() || self.rev_ptr != SENTINEL) {
            if self.rev_ptr != SENTINEL {
                self.rev_stack.push(self.rev_ptr);
                self.rev_ptr = self.tree.get_right(self.rev_ptr);
            } else {
                let current_node = self.rev_stack.pop();
                if current_node == self.fwd_node {
                    self.terminated = true;
                    return None;
                }
                self.rev_node = current_node;
                let node = self.tree.get_node(current_node.unwrap());
                self.rev_ptr = self.tree.get_left(current_node.unwrap());
                return Some((&node.key, &node.value));
            }
        }
        None
    }
}

pub struct RedBlackTreeIteratorMut<
    'a,
    K: Debug + PartialOrd + Ord + Copy + Clone + Default + Pod + Zeroable,
    V: Default + Copy + Clone + Pod + Zeroable,
    const MAX_SIZE: usize,
> {
    tree: &'a mut RedBlackTree<K, V, MAX_SIZE>,
    fwd_stack: Vec<u32>,
    fwd_ptr: u32,
    fwd_node: Option<u32>,
    rev_stack: Vec<u32>,
    rev_ptr: u32,
    rev_node: Option<u32>,
    terminated: bool,
}

impl<
        'a,
        K: Debug + PartialOrd + Ord + Copy + Clone + Default + Pod + Zeroable,
        V: Default + Copy + Clone + Pod + Zeroable,
        const MAX_SIZE: usize,
    > Iterator for RedBlackTreeIteratorMut<'a, K, V, MAX_SIZE>
{
    type Item = (&'a K, &'a mut V);

    fn next(&mut self) -> Option<Self::Item> {
        while !self.terminated && (!self.fwd_stack.is_empty() || self.fwd_ptr != SENTINEL) {
            if self.fwd_ptr != SENTINEL {
                self.fwd_stack.push(self.fwd_ptr);
                self.fwd_ptr = self.tree.get_left(self.fwd_ptr);
            } else {
                let current_node = self.fwd_stack.pop();
                if current_node == self.rev_node {
                    self.terminated = true;
                    return None;
                }
                self.fwd_node = current_node;
                let ptr = self.fwd_node.unwrap();
                self.fwd_ptr = self.tree.get_right(ptr);
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
        K: Debug + PartialOrd + Ord + Copy + Clone + Default + Pod + Zeroable,
        V: Default + Copy + Clone + Pod + Zeroable,
        const MAX_SIZE: usize,
    > DoubleEndedIterator for RedBlackTreeIteratorMut<'a, K, V, MAX_SIZE>
{
    fn next_back(&mut self) -> Option<Self::Item> {
        while !self.terminated && (!self.rev_stack.is_empty() || self.rev_ptr != SENTINEL) {
            if self.rev_ptr != SENTINEL {
                self.rev_stack.push(self.rev_ptr);
                self.rev_ptr = self.tree.get_right(self.rev_ptr);
            } else {
                let current_node = self.rev_stack.pop();
                if current_node == self.fwd_node {
                    self.terminated = true;
                    return None;
                }
                self.rev_node = current_node;
                let ptr = self.rev_node.unwrap();
                self.rev_ptr = self.tree.get_left(ptr);
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
        K: Debug + PartialOrd + Ord + Copy + Clone + Default + Pod + Zeroable,
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
        K: Debug + PartialOrd + Ord + Copy + Clone + Default + Pod + Zeroable,
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
    type Rbt = RedBlackTree<u64, u64, 1024>;
    let mut buf = vec![0u8; std::mem::size_of::<Rbt>()];
    let tree = Rbt::new_from_slice(buf.as_mut_slice());
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
    type Rbt = RedBlackTree<u64, u64, 1024>;
    let mut buf = vec![0u8; std::mem::size_of::<Rbt>()];
    let tree = Rbt::new_from_slice(buf.as_mut_slice());
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
    type Rbt = RedBlackTree<u64, u64, 1024>;
    let mut buf = vec![0u8; std::mem::size_of::<Rbt>()];
    let tree = Rbt::new_from_slice(buf.as_mut_slice());
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
    type Rbt = RedBlackTree<u64, u64, 1024>;
    let mut buf = vec![0u8; std::mem::size_of::<Rbt>()];
    let tree = Rbt::new_from_slice(buf.as_mut_slice());
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
    type Rbt = RedBlackTree<u64, u64, 1024>;
    let mut buf = vec![0u8; std::mem::size_of::<Rbt>()];
    let tree = Rbt::new_from_slice(buf.as_mut_slice());
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
    tree.pretty_print();
}

/// Test a power of 2 minus 1
#[test]
fn test_delete_multiple_random_1023() {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    type Rbt = RedBlackTree<u64, u64, 1023>;
    let mut buf = vec![0u8; std::mem::size_of::<Rbt>()];
    let tree = Rbt::new_from_slice(buf.as_mut_slice());
    let mut keys = vec![];
    // Fill up tree
    for k in 0..1023 {
        let mut hasher = DefaultHasher::new();
        (k as u64).hash(&mut hasher);
        let key = hasher.finish();
        tree.insert(key, 0).unwrap();
        keys.push(key);
        assert!(tree.is_valid_red_black_tree());
    }

    for i in keys.iter() {
        tree.remove(i).unwrap();
        assert!(tree.is_valid_red_black_tree());
    }
}

#[test]
fn test_delete_multiple_random_1024() {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    type Rbt = RedBlackTree<u64, u64, 1024>;
    let mut buf = vec![0u8; std::mem::size_of::<Rbt>()];
    let tree = Rbt::new_from_slice(buf.as_mut_slice());
    let mut keys = vec![];
    let mut addrs = vec![];
    // Fill up tree
    for k in 0..1024 {
        let mut hasher = DefaultHasher::new();
        (k as u64).hash(&mut hasher);
        let key = hasher.finish();
        addrs.push(tree.insert(key, 0).unwrap());
        keys.push(key);
        assert!(tree.is_valid_red_black_tree());
    }

    for (k, a) in keys.iter().zip(addrs) {
        assert!(tree.get_addr(k) == a);
    }

    for i in keys.iter() {
        tree.remove(i).unwrap();
        assert!(tree.is_valid_red_black_tree());
    }
}

#[test]
fn test_delete_multiple_random_2048() {
    use std::collections::{hash_map::DefaultHasher, BTreeMap};
    use std::hash::{Hash, Hasher};
    type Rbt = RedBlackTree<u64, u64, 2048>;
    let mut buf = vec![0u8; std::mem::size_of::<Rbt>()];
    let tree = Rbt::new_from_slice(buf.as_mut_slice());
    let mut keys = vec![];
    // Fill up tree
    for k in 0..2048 {
        let mut hasher = DefaultHasher::new();
        (k as u64).hash(&mut hasher);
        let key = hasher.finish();
        tree.insert(key, 0).unwrap();
        keys.push(key);
    }

    let key_to_index = keys
        .iter()
        .enumerate()
        .map(|(i, k)| (*k, i as u64))
        .collect::<BTreeMap<_, _>>();

    let mut buf = vec![0u8; std::mem::size_of::<Rbt>()];
    let index_tree = Rbt::new_from_slice(buf.as_mut_slice());
    let mut index_keys = vec![];

    for k in keys.iter() {
        let key = key_to_index[k];
        index_tree.insert(key, 0).unwrap();
        index_keys.push(key);
    }

    assert!(index_tree.is_valid_red_black_tree());
    for i in index_keys.iter() {
        index_tree.remove(i).unwrap();
        assert!(index_tree.is_valid_red_black_tree());
    }
}

#[test]
fn test_delete_multiple_random_512() {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    type Rbt = RedBlackTree<u64, u64, 512>;
    let mut buf = vec![0u8; std::mem::size_of::<Rbt>()];
    let tree = Rbt::new_from_slice(buf.as_mut_slice());
    let mut keys = vec![];
    // Fill up tree
    for k in 0..512 {
        let mut hasher = DefaultHasher::new();
        (k as u64).hash(&mut hasher);
        let key = hasher.finish();
        tree.insert(key, 0).unwrap();
        keys.push(key);
        assert!(tree.is_valid_red_black_tree());
    }
    for i in keys.iter() {
        tree.remove(i).unwrap();
        assert!(tree.is_valid_red_black_tree());
    }
}

#[test]
fn test_delete_multiple_random_4098() {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    type Rbt = RedBlackTree<u64, u64, 4098>;
    let mut buf = vec![0u8; std::mem::size_of::<Rbt>()];
    let tree = Rbt::new_from_slice(buf.as_mut_slice());
    let mut keys = vec![];
    // Fill up tree
    for k in 0..4098 {
        let mut hasher = DefaultHasher::new();
        (k as u64).hash(&mut hasher);
        let key = hasher.finish();
        tree.insert(key, 0).unwrap();
        keys.push(key);
        assert!(tree.is_valid_red_black_tree());
    }
    for i in keys.iter() {
        tree.remove(i).unwrap();
        assert!(tree.is_valid_red_black_tree());
    }
}
