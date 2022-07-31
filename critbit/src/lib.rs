use bytemuck::{Pod, Zeroable};
use node_allocator::{NodeAllocator, ZeroCopy, SENTINEL};
use std::ops::{Index, IndexMut};

// Register aliases
pub const LEFT: u32 = 0;
pub const RIGHT: u32 = 1;
pub const PARENT: u32 = 2;
pub const VALUE: u32 = 3;

#[repr(C)]
#[derive(Default, Copy, Clone)]
pub struct CritbitNode {
    prefix_len: u64,
    key: u128,
}

unsafe impl Zeroable for CritbitNode {}
unsafe impl Pod for CritbitNode {}

impl CritbitNode {
    pub fn new(prefix_len: u64, key: u128) -> Self {
        Self { prefix_len, key }
    }
}

#[derive(Copy, Clone)]
pub struct Critbit<
    const MAX_SIZE: usize,
    const MAX_LEAVES: usize,
    V: Default + Copy + Clone + Pod + Zeroable,
> {
    pub sequence_number: u64,
    pub root: u32,
    pub num_leaves: u32,
    node_allocator: NodeAllocator<MAX_SIZE, 4, CritbitNode>,
    leaves: NodeAllocator<MAX_LEAVES, 1, V>,
}

unsafe impl<
        const MAX_SIZE: usize,
        const MAX_LEAVES: usize,
        V: Default + Copy + Clone + Pod + Zeroable,
    > Zeroable for Critbit<MAX_SIZE, MAX_LEAVES, V>
{
}

unsafe impl<
        const MAX_SIZE: usize,
        const MAX_LEAVES: usize,
        V: Default + Copy + Clone + Pod + Zeroable,
    > Pod for Critbit<MAX_SIZE, MAX_LEAVES, V>
{
}

impl<
        const MAX_SIZE: usize,
        const MAX_LEAVES: usize,
        V: Default + Copy + Clone + Pod + Zeroable,
    > ZeroCopy for Critbit<MAX_SIZE, MAX_LEAVES, V>
{
}

impl<
        const MAX_SIZE: usize,
        const MAX_LEAVES: usize,
        V: Default + Copy + Clone + Pod + Zeroable,
    > Default for Critbit<MAX_SIZE, MAX_LEAVES, V>
{
    fn default() -> Self {
        assert!(MAX_SIZE >= 2 * MAX_LEAVES);
        Self {
            sequence_number: 0,
            root: SENTINEL,
            num_leaves: 0,
            node_allocator: NodeAllocator::<MAX_SIZE, 4, CritbitNode>::default(),
            leaves: NodeAllocator::<MAX_LEAVES, 1, V>::default(),
        }
    }
}

impl<
        const MAX_SIZE: usize,
        const MAX_LEAVES: usize,
        V: Default + Copy + Clone + Pod + Zeroable,
    > Critbit<MAX_SIZE, MAX_LEAVES, V>
{
    pub fn new() -> Self {
        Self::default()
    }

    pub fn new_from_slice(slice: &mut [u8]) -> &mut Self {
        let tree = Self::load_mut_bytes(slice).unwrap();
        tree.node_allocator.init_default();
        tree.leaves.init_default();
        tree
    }

    pub fn size(&self) -> usize {
        self.num_leaves as usize
    }

    pub fn get_leaf(&self, leaf_index: u32) -> &V {
        self.leaves.get(leaf_index).get_value()
    }

    pub fn get_leaf_mut(&mut self, leaf_index: u32) -> &mut V {
        self.leaves.get_mut(leaf_index).get_value_mut()
    }

    fn get_leaf_index(&self, node: u32) -> u32 {
        self.node_allocator.get_register(node, VALUE)
    }

    pub fn is_inner_node(&self, node: u32) -> bool {
        self.node_allocator.get_register(node, VALUE) == SENTINEL
    }

    pub fn get_node(&self, node: u32) -> CritbitNode {
        *self.node_allocator.get(node).get_value()
    }

    #[inline(always)]
    pub fn get_left(&self, node: u32) -> u32 {
        self.node_allocator.get_register(node, LEFT)
    }

    #[inline(always)]
    pub fn get_right(&self, node: u32) -> u32 {
        self.node_allocator.get_register(node, RIGHT)
    }

    #[inline(always)]
    pub fn get_parent(&self, node: u32) -> u32 {
        self.node_allocator.get_register(node, PARENT)
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
            .set_register(node_index, leaf_index, VALUE);
        self.leaves.get_mut(leaf_index).set_value(value);
        (node_index, leaf_index)
    }

    #[inline(always)]
    fn get_child(&self, prefix_len: u64, node_index: u32, search_key: u128) -> (u32, bool) {
        let crit_bit_mask = (1u128 << 127) >> prefix_len;
        if (search_key & crit_bit_mask) != 0 {
            (self.node_allocator.get_register(node_index, RIGHT), true)
        } else {
            (self.node_allocator.get_register(node_index, LEFT), false)
        }
    }

    #[inline(always)]
    fn duplicate(&mut self, node_index: u32) -> u32 {
        let index = self.node_allocator.add_node(self.get_node(node_index));
        let left = self.get_left(node_index);
        let right = self.get_right(node_index);
        let value = self.node_allocator.get_register(node_index, VALUE);
        self.node_allocator.set_register(index, value, VALUE);
        self.node_allocator.connect(index, left, LEFT, PARENT);
        self.node_allocator.connect(index, right, RIGHT, PARENT);
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
        self.node_allocator.clear_register(node_index, VALUE);
        self.node_allocator.connect(node_index, left, LEFT, PARENT);
        self.node_allocator
            .connect(node_index, right, RIGHT, PARENT);
    }

    #[inline(always)]
    fn migrate(&mut self, source: u32, target: u32) {
        let content = self.get_node(source);
        *self.get_node_mut(target) = content;
        if !self.is_inner_node(source) {
            assert!(self.get_left(source) == SENTINEL);
            assert!(self.get_right(source) == SENTINEL);
            let leaf_index = self.get_leaf_index(source);
            self.node_allocator.clear_register(source, VALUE);
            self.node_allocator.set_register(target, leaf_index, VALUE);
        }
        assert!(self.get_leaf_index(source) == SENTINEL);
        self.node_allocator
            .connect(target, self.get_left(source), LEFT, PARENT);
        self.node_allocator
            .connect(target, self.get_right(source), RIGHT, PARENT);
        self.node_allocator.clear_register(source, LEFT);
        self.node_allocator.clear_register(source, RIGHT);
        self.node_allocator.remove_node(source);
    }

    #[inline(always)]
    fn remove_leaf(&mut self, node_index: u32) -> V {
        let leaf_index = self.get_leaf_index(node_index);
        let value = *self.get_leaf(leaf_index);
        self.node_allocator.clear_register(node_index, VALUE);
        assert!(self.get_leaf_index(node_index) == SENTINEL);
        let parent = self.get_parent(node_index);
        if node_index == self.get_left(parent) {
            self.node_allocator
                .disconnect(node_index, parent, PARENT, LEFT);
        } else if node_index == self.get_right(parent) {
            self.node_allocator
                .disconnect(node_index, parent, PARENT, RIGHT);
        } else if parent != SENTINEL {
            panic!("Parent is not connected to child");
        }
        self.leaves.remove_node(leaf_index);
        self.node_allocator.remove_node(node_index);
        value
    }

    pub fn get(&self, key: u128) -> Option<&V> {
        let mut node_index = self.root;
        loop {
            let node = self.get_node(node_index);
            if !self.is_inner_node(node_index) {
                if node.key == key {
                    let leaf_index = self.get_leaf_index(node_index);
                    return Some(&self.get_leaf(leaf_index));
                } else {
                    return None;
                }
            }
            let shared_prefix_len = (node.key ^ key).leading_zeros() as u64;
            if shared_prefix_len >= node.prefix_len {
                node_index = self.get_child(node.prefix_len, node_index, key).0;
                continue;
            }
        }
    }

    pub fn get_mut(&mut self, key: u128) -> Option<&mut V> {
        let mut node_index = self.root;
        loop {
            let node = self.get_node(node_index);
            if !self.is_inner_node(node_index) {
                if node.key == key {
                    let leaf_index = self.get_leaf_index(node_index);
                    return Some(self.get_leaf_mut(leaf_index));
                } else {
                    return None;
                }
            }
            let shared_prefix_len = (node.key ^ key).leading_zeros() as u64;
            if shared_prefix_len >= node.prefix_len {
                node_index = self.get_child(node.prefix_len, node_index, key).0;
                continue;
            }
        }
    }

    pub fn insert(&mut self, key: u128, value: V) -> Option<(u32, u32)> {
        assert!(self.num_leaves as usize == self.leaves.size as usize);
        if self.root == SENTINEL {
            let (node_index, leaf_index) = self.add_leaf(key, value);
            self.root = node_index;
            self.sequence_number += 1;
            self.num_leaves += 1;
            return Some((self.root, leaf_index));
        }
        let mut node_index = self.root;
        loop {
            let node = self.get_node(node_index);
            if node.key == key && !self.is_inner_node(node_index) {
                // Replace the node with the new value
                let leaf_index = self.get_leaf_index(node_index);
                self.replace_leaf(leaf_index, value);
                return Some((node_index, leaf_index));
            }
            let shared_prefix_len = (node.key ^ key).leading_zeros() as u64;
            if shared_prefix_len >= node.prefix_len {
                node_index = self.get_child(node.prefix_len, node_index, key).0;
                continue;
            }
            let crit_bit_mask: u128 = (1u128 << 127) >> shared_prefix_len;
            let is_right = (crit_bit_mask & key) != 0;
            let (node_leaf_index, leaf_index) = self.add_leaf(key, value);
            let moved_node_index = self.duplicate(node_index);
            let new_node = CritbitNode::new(shared_prefix_len, key);
            if is_right {
                self.replace_node(node_index, &new_node, moved_node_index, node_leaf_index);
            } else {
                self.replace_node(node_index, &new_node, node_leaf_index, moved_node_index);
            }
            self.sequence_number += 1;
            self.num_leaves += 1;
            return Some((node_leaf_index, leaf_index));
        }
    }

    pub fn remove(&mut self, key: u128) -> Option<V> {
        assert!(self.num_leaves as usize == self.leaves.size as usize);
        let nsize = self.node_allocator.size;
        let lsize = self.leaves.size;
        let mut parent = self.root;
        let mut child: u32;
        let mut is_right: bool;
        if self.is_inner_node(parent) {
            let node = self.get_node(parent);
            (child, is_right) = self.get_child(node.prefix_len, parent, key);
        } else {
            let leaf = self.get_node(parent);
            if leaf.key == key {
                self.root = SENTINEL;
                assert!(self.num_leaves == 1);
                self.num_leaves = 0;
                return Some(self.remove_leaf(parent));
            } else {
                return None;
            }
        }
        loop {
            let node = self.get_node(child);
            if self.is_inner_node(child) {
                let (grandchild, grandchild_crit_bit) = self.get_child(node.prefix_len, child, key);
                parent = child;
                child = grandchild;
                is_right = grandchild_crit_bit;
            } else {
                if node.key != key {
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
        self.num_leaves -= 1;
        self.sequence_number += 1;
        assert!(nsize - self.node_allocator.size == 2);
        assert!(lsize - self.leaves.size == 1);
        Some(leaf)
    }

    pub fn inorder_traversal(&self) -> Vec<(u128, V)> {
        let mut stack = vec![self.root];
        let mut leaves = vec![];
        while !stack.is_empty() {
            let node = stack.pop();
            match node {
                Some(n) => {
                    if !self.is_inner_node(n) {
                        let i = self.get_leaf_index(n);
                        let v = self.get_leaf(i);
                        leaves.push((self.get_node(n).key, *v))
                    } else {
                        stack.push(self.get_right(n));
                        stack.push(self.get_left(n));
                    }
                }
                None => return leaves,
            }
        }
        leaves
    }
}

impl<
        const MAX_SIZE: usize,
        const MAX_LEAVES: usize,
        V: Default + Copy + Clone + Pod + Zeroable,
    > Index<u128> for Critbit<MAX_SIZE, MAX_LEAVES, V>
{
    type Output = V;

    fn index(&self, index: u128) -> &Self::Output {
        &self.get(index).unwrap()
    }
}

impl<
        const MAX_SIZE: usize,
        const MAX_LEAVES: usize,
        V: Default + Copy + Clone + Pod + Zeroable,
    > IndexMut<u128> for Critbit<MAX_SIZE, MAX_LEAVES, V>
{
    fn index_mut(&mut self, index: u128) -> &mut Self::Output {
        self.get_mut(index).unwrap()
    }
}
