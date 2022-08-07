use bytemuck::{Pod, Zeroable};
use node_allocator::{NodeAllocator, ZeroCopy, SENTINEL};
use std::{
    cmp::max,
    ops::{Index, IndexMut},
};

// The number of registers (the last register is currently not in use).
const REGISTERS: usize = 4;

// Enum representing the fields of a node:
// 0 - left pointer
// 1 - right pointer
// 2 - height of the (sub-)tree
#[derive(Debug, Copy, Clone, PartialEq)]
pub enum Field {
    Left = 0,
    Right = 1,
    Height = 2,
}

// Type representing a path entry (parent, branch, child) when
// traversing the tree.
type Ancestor = (Option<u32>, Option<Field>, u32);

#[repr(C)]
#[derive(Default, Copy, Clone)]
pub struct AVLNode<
    K: PartialOrd + Copy + Clone + Default + Pod + Zeroable,
    V: Default + Copy + Clone + Pod + Zeroable,
> {
    pub key: K,
    pub value: V,
}

unsafe impl<
        K: PartialOrd + Copy + Clone + Default + Pod + Zeroable,
        V: Default + Copy + Clone + Pod + Zeroable,
    > Zeroable for AVLNode<K, V>
{
}
unsafe impl<
        K: PartialOrd + Copy + Clone + Default + Pod + Zeroable,
        V: Default + Copy + Clone + Pod + Zeroable,
    > Pod for AVLNode<K, V>
{
}

impl<
        K: PartialOrd + Copy + Clone + Default + Pod + Zeroable,
        V: Default + Copy + Clone + Pod + Zeroable,
    > AVLNode<K, V>
{
    pub fn new(key: K, value: V) -> Self {
        Self { key, value }
    }
}

#[derive(Copy, Clone)]
pub struct AVLTree<
    K: PartialOrd + Copy + Clone + Default + Pod + Zeroable,
    V: Default + Copy + Clone + Pod + Zeroable,
    const MAX_SIZE: usize,
> {
    pub root: u32,

    allocator: NodeAllocator<AVLNode<K, V>, MAX_SIZE, REGISTERS>,
}

unsafe impl<
        K: PartialOrd + Copy + Clone + Default + Pod + Zeroable,
        V: Default + Copy + Clone + Pod + Zeroable,
        const MAX_SIZE: usize,
    > Zeroable for AVLTree<K, V, MAX_SIZE>
{
}
unsafe impl<
        K: PartialOrd + Copy + Clone + Default + Pod + Zeroable,
        V: Default + Copy + Clone + Pod + Zeroable,
        const MAX_SIZE: usize,
    > Pod for AVLTree<K, V, MAX_SIZE>
{
}

impl<
        K: PartialOrd + Copy + Clone + Default + Pod + Zeroable,
        V: Default + Copy + Clone + Pod + Zeroable,
        const MAX_SIZE: usize,
    > ZeroCopy for AVLTree<K, V, MAX_SIZE>
{
}

impl<
        K: PartialOrd + Copy + Clone + Default + Pod + Zeroable,
        V: Default + Copy + Clone + Pod + Zeroable,
        const MAX_SIZE: usize,
    > Default for AVLTree<K, V, MAX_SIZE>
{
    fn default() -> Self {
        AVLTree {
            root: SENTINEL,
            allocator: NodeAllocator::<AVLNode<K, V>, MAX_SIZE, REGISTERS>::default(),
        }
    }
}

impl<
        K: PartialOrd + Copy + Clone + Default + Pod + Zeroable,
        V: Default + Copy + Clone + Pod + Zeroable,
        const MAX_SIZE: usize,
    > AVLTree<K, V, MAX_SIZE>
{
    pub fn size(&self) -> usize {
        self.allocator.size as usize
    }

    pub fn new() -> Self {
        Self::default()
    }

    pub fn new_from_slice(slice: &mut [u8]) -> &mut Self {
        let tree = Self::load_mut_bytes(slice).unwrap();
        tree.allocator.init_default();
        tree
    }

    pub fn get_node(&self, node: u32) -> &AVLNode<K, V> {
        self.allocator.get(node).get_value()
    }

    fn get_node_mut(&mut self, node: u32) -> &mut AVLNode<K, V> {
        self.allocator.get_mut(node).get_value_mut()
    }

    #[inline(always)]
    fn set_field(&mut self, node: u32, register: Field, value: u32) {
        if node != SENTINEL {
            self.allocator.set_register(node, value, register as u32);
        }
    }

    #[inline(always)]
    fn get_field(&self, node: u32, register: Field) -> u32 {
        self.allocator.get_register(node, register as u32)
    }

    pub fn insert(&mut self, key: K, value: V) -> Option<u32> {
        let mut reference_node = self.root;
        let new_node = AVLNode::<K, V>::new(key, value);
        if reference_node == SENTINEL {
            self.root = self.allocator.add_node(new_node);
            return Some(self.root);
        }

        let mut path: Vec<Ancestor> = Vec::with_capacity((self.size() as f64).log2() as usize);
        path.push((None, None, reference_node));

        loop {
            let current_key = self.get_node(reference_node).key;
            let parent = reference_node;

            let branch = if key < current_key {
                reference_node = self.get_field(parent, Field::Left);
                Field::Left
            } else if key > current_key {
                reference_node = self.get_field(parent, Field::Right);
                Field::Right
            } else {
                self.get_node_mut(reference_node).value = value;
                return Some(reference_node);
            };

            if reference_node == SENTINEL {
                if self.size() >= MAX_SIZE - 1 {
                    return None;
                }
                reference_node = self.allocator.add_node(new_node);
                self.set_field(parent, branch, reference_node);
                break;
            } else {
                path.push((Some(parent), Some(branch), reference_node));
            }
        }

        self.rebalance(path);

        Some(reference_node)
    }

    pub fn remove(&mut self, key: &K) -> Option<V> {
        let mut node_index = self.root;
        if node_index == SENTINEL {
            return None;
        }

        let mut path: Vec<Ancestor> = Vec::with_capacity((self.size() as f64).log2() as usize);
        path.push((None, None, node_index));

        while node_index != SENTINEL {
            let current_key = self.get_node(node_index).key;
            let parent = node_index;

            let branch = if *key < current_key {
                node_index = self.get_field(parent, Field::Left);
                Field::Left
            } else if *key > current_key {
                node_index = self.get_field(parent, Field::Right);
                Field::Right
            } else {
                break;
            };

            path.push((Some(parent), Some(branch), node_index));
        }
        // sanity check: the loop should be stopped by the break statement
        // node_index == SENTINEL indicates that the key was not found
        if node_index == SENTINEL {
            return None;
        }

        let value = self.allocator.get(node_index).get_value().value;
        let left = self.get_field(node_index, Field::Left);
        let right = self.get_field(node_index, Field::Right);

        let replacement = if left != SENTINEL && right != SENTINEL {
            let mut leftmost = right;
            let mut leftmost_parent = SENTINEL;
            // path to the leftmost descendant
            let mut inner_path = Vec::with_capacity((self.size() as f64).log2() as usize);

            while self.get_field(leftmost, Field::Left) != SENTINEL {
                leftmost_parent = leftmost;
                leftmost = self.get_field(leftmost, Field::Left);
                inner_path.push((Some(leftmost_parent), Some(Field::Left), leftmost));
            }
            
            if leftmost_parent != SENTINEL {
                self.set_field(
                    leftmost_parent,
                    Field::Left,
                    self.get_field(leftmost, Field::Right),
                );
            }

            self.set_field(leftmost, Field::Left, left);
            if right != leftmost {
                self.set_field(leftmost, Field::Right, right);
            }

            let (parent, branch, _) = path.pop().unwrap();

            if let Some(parent) = parent {
                self.set_field(parent, branch.unwrap(), leftmost);
            }

            path.push((parent, branch, leftmost));
            if right != leftmost {
                path.push((Some(leftmost), Some(Field::Right), right));
            }
            // drop the last inner_path element since it references the leftmost node
            if !inner_path.is_empty() {
                inner_path.pop();
            }
            path.extend(inner_path);

            leftmost
        } else {
            let child = if left == SENTINEL && right == SENTINEL {
                SENTINEL
            } else {
                if left != SENTINEL { left } else { right }
            };

            let (parent, branch, _) = path.pop().unwrap();

            if let Some(parent) = parent {
                self.set_field(parent, branch.unwrap(), child);

                if child != SENTINEL {
                    path.push((Some(parent), branch, child));
                }
            }

            child
        };

        if node_index == self.root {
            self.root = replacement;
        }

        self.delete(node_index);
        self.rebalance(path);

        Some(value)
    }

    fn balance_factor(&self, left: u32, right: u32) -> i32 {
        // safe to convert to i32 since height will be at most log2(capacity)
        let left_height = if left != SENTINEL {
            self.get_field(left, Field::Height) as i32 + 1
        } else {
            0
        };
        let right_height = if right != SENTINEL {
            self.get_field(right, Field::Height) as i32 + 1
        } else {
            0
        };

        left_height - right_height
    }

    fn left_rotate(&mut self, index: u32) -> u32 {
        let right = self.get_field(index, Field::Right);
        let right_left = self.get_field(right, Field::Left);

        self.set_field(index, Field::Right, right_left);
        self.set_field(right, Field::Left, index);

        right
    }

    fn right_rotate(&mut self, index: u32) -> u32 {
        let left = self.get_field(index, Field::Left);
        let left_right = self.get_field(left, Field::Right);

        self.set_field(index, Field::Left, left_right);
        self.set_field(left, Field::Right, index);

        left
    }

    fn update_height(&mut self, index: u32) {
        let left = self.get_field(index, Field::Left);
        let right = self.get_field(index, Field::Right);

        let height = if (left + right) == 0 {
            0
        } else {
            let left_height = if left != SENTINEL {
                self.get_field(left, Field::Height)
            } else {
                0
            };
            let right_height = if right != SENTINEL {
                self.get_field(right, Field::Height)
            } else {
                0
            };

            max(left_height, right_height) + 1
        };

        self.set_field(index, Field::Height, height);
    }

    fn delete(&mut self, node: u32) {
        self.allocator.clear_register(node, Field::Left as u32);
        self.allocator.clear_register(node, Field::Right as u32);
        self.allocator.clear_register(node, Field::Height as u32);
        self.allocator.remove_node(node);
    }

    fn rebalance(&mut self, path: Vec<Ancestor>) {
        for (parent, branch, child) in path.iter().rev() {
            let left = self.get_field(*child, Field::Left);
            let right = self.get_field(*child, Field::Right);

            let balance_factor = self.balance_factor(left, right);

            let index = if balance_factor > 1 {
                let left_left = self.get_field(left, Field::Left);
                let left_right = self.get_field(left, Field::Right);
                let left_balance_factor = self.balance_factor(left_left, left_right);

                if left_balance_factor < 0 {
                    let index = self.left_rotate(left);
                    self.set_field(*child, Field::Left, index);
                }

                Some(self.right_rotate(*child))
            } else if balance_factor < -1 {
                let right_left = self.get_field(right, Field::Left);
                let right_right = self.get_field(right, Field::Right);
                let right_balance_factor = self.balance_factor(right_left, right_right);

                if right_balance_factor > 0 {
                    let index = self.right_rotate(right);
                    self.set_field(*child, Field::Right, index);
                }

                Some(self.left_rotate(*child))
            } else {
                self.update_height(*child);
                None
            };
            if let Some(index) = index {
                if let Some(parent) = parent {
                    self.set_field(*parent, (*branch).unwrap(), index);
                } else {
                    self.root = index;
                    self.update_height(index);
                }
            }
        }
    }

    pub fn get(&self, key: &K) -> Option<&V> {
        let mut reference_node = self.root;
        if reference_node == SENTINEL {
            return None;
        }
        loop {
            let ref_value = self.allocator.get(reference_node).get_value().key;
            let target = if *key < ref_value {
                self.get_field(reference_node, Field::Left)
            } else if *key > ref_value {
                self.get_field(reference_node, Field::Right)
            } else {
                return Some(&self.get_node(reference_node).value);
            };
            if target == SENTINEL {
                return None;
            }
            reference_node = target
        }
    }

    pub fn get_mut(&mut self, key: &K) -> Option<&mut V> {
        let mut reference_node = self.root;
        if reference_node == SENTINEL {
            return None;
        }
        loop {
            let ref_value = self.allocator.get(reference_node).get_value().key;
            let target = if *key < ref_value {
                self.get_field(reference_node, Field::Left)
            } else if *key > ref_value {
                self.get_field(reference_node, Field::Right)
            } else {
                return Some(&mut self.get_node_mut(reference_node).value);
            };
            if target == SENTINEL {
                return None;
            }
            reference_node = target
        }
    }

    pub fn min(&self) -> Option<&V> {
        if self.root == SENTINEL {
            return None;
        }
        let mut node = self.root;
        while self.get_field(node, Field::Left) != SENTINEL {
            node = self.get_field(node, Field::Left);
        }
        return Some(&self.get_node(node).value);
    }

    pub fn max(&self) -> Option<&V> {
        if self.root == SENTINEL {
            return None;
        }
        let mut node = self.root;
        while self.get_field(node, Field::Right) != SENTINEL {
            node = self.get_field(node, Field::Right);
        }
        return Some(&self.get_node(node).value);
    }

    pub fn iter(&self) -> AVLTreeIterator<'_, K, V, MAX_SIZE> {
        AVLTreeIterator::<K, V, MAX_SIZE> {
            tree: self,
            stack: vec![],
            node: self.root,
        }
    }

    pub fn iter_mut(&mut self) -> AVLTreeIteratorMut<'_, K, V, MAX_SIZE> {
        let node = self.root;
        AVLTreeIteratorMut::<K, V, MAX_SIZE> {
            tree: self,
            stack: vec![],
            node,
        }
    }
}

pub struct AVLTreeIterator<
    'a,
    K: PartialOrd + Copy + Clone + Default + Pod + Zeroable,
    V: Default + Copy + Clone + Pod + Zeroable,
    const MAX_SIZE: usize,
> {
    pub tree: &'a AVLTree<K, V, MAX_SIZE>,
    pub stack: Vec<u32>,
    pub node: u32,
}

impl<
        'a,
        K: PartialOrd + Copy + Clone + Default + Pod + Zeroable,
        V: Default + Copy + Clone + Pod + Zeroable,
        const MAX_SIZE: usize,
    > Iterator for AVLTreeIterator<'a, K, V, MAX_SIZE>
{
    type Item = (&'a K, &'a V);

    fn next(&mut self) -> Option<Self::Item> {
        while !self.stack.is_empty() || self.node != SENTINEL {
            if self.node != SENTINEL {
                self.stack.push(self.node);
                self.node = self.tree.get_field(self.node, Field::Left);
            } else {
                self.node = self.stack.pop().unwrap();
                let node = self.tree.get_node(self.node);
                self.node = self.tree.get_field(self.node, Field::Right);
                return Some((&node.key, &node.value));
            }
        }
        return None;
    }
}

pub struct AVLTreeIteratorMut<
    'a,
    K: PartialOrd + Copy + Clone + Default + Pod + Zeroable,
    V: Default + Copy + Clone + Pod + Zeroable,
    const MAX_SIZE: usize,
> {
    pub tree: &'a mut AVLTree<K, V, MAX_SIZE>,
    pub stack: Vec<u32>,
    pub node: u32,
}

impl<
        'a,
        K: PartialOrd + Copy + Clone + Default + Pod + Zeroable,
        V: Default + Copy + Clone + Pod + Zeroable,
        const MAX_SIZE: usize,
    > Iterator for AVLTreeIteratorMut<'a, K, V, MAX_SIZE>
{
    type Item = (&'a K, &'a mut V);

    fn next(&mut self) -> Option<Self::Item> {
        while !self.stack.is_empty() || self.node != SENTINEL {
            if self.node != SENTINEL {
                self.stack.push(self.node);
                self.node = self.tree.get_field(self.node, Field::Left);
            } else {
                self.node = self.stack.pop().unwrap();
                let ptr = self.node;
                self.node = self.tree.get_field(ptr, Field::Right);
                // TODO: How does one remove this unsafe?
                unsafe {
                    let node =
                        (*self.tree.allocator.nodes.as_mut_ptr().add(ptr as usize)).get_value_mut();
                    return Some((&node.key, &mut node.value));
                }
            }
        }
        return None;
    }
}

impl<
        K: PartialOrd + Copy + Clone + Default + Pod + Zeroable,
        V: Default + Copy + Clone + Pod + Zeroable,
        const MAX_SIZE: usize,
    > Index<&K> for AVLTree<K, V, MAX_SIZE>
{
    type Output = V;

    fn index(&self, index: &K) -> &Self::Output {
        &self.get(index).unwrap()
    }
}

impl<
        const MAX_SIZE: usize,
        K: PartialOrd + Copy + Clone + Default + Pod + Zeroable,
        V: Default + Copy + Clone + Pod + Zeroable,
    > IndexMut<&K> for AVLTree<K, V, MAX_SIZE>
{
    fn index_mut(&mut self, index: &K) -> &mut Self::Output {
        self.get_mut(index).unwrap()
    }
}
