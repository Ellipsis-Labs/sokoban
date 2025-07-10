use num_derive::FromPrimitive;

/// Enum representing the fields of a tree node:
/// 0 - left pointer
/// 1 - right pointer
/// 2 - parent pointer
/// 3 - value pointer (index of leaf)
#[derive(Debug, Copy, Clone, PartialEq, Eq, FromPrimitive)]
pub enum TreeField {
    Left = 0,
    Right = 1,
    Parent = 2,
    Value = 3,
}

/// Enum representing the fields of a simple node (Linked List / Binary Tree):
/// 0 - left pointer
/// 1 - right pointer
#[derive(Debug, Copy, Clone, PartialEq, Eq, FromPrimitive)]
pub enum NodeField {
    Left = 0,
    Right = 1,
}
