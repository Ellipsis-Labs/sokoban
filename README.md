# Sokoban
Compact, efficient data structures in contiguous byte arrays.

## DISCLAIMER
This is unaudited code, and still very rough around the edges. Proceed with caution before using any of these data structures in a production system. I highly encourage feedback, suggestions, issues, and PRs :smiley:

### Why compact data structures?
For most applications, there is no reason to look past the Rust standard library for data structures. However, when the application has limited or expensive memory and is bottlenecked by performance, programmers will often need to design custom solutions to address those constraints. These types of constraints come up quite frequently in high frequency trading, embedded systems, and blockchain development.

Enter Sokoban: A library of data structures designed to simplify this exact problem.

### Generic Node Allocator
Almost all data structures can be represented by some sort of connected graph of nodes and edges. The `node-allocator` module implements a raw node allocation data structure for contiguous buffers. Each entry in the buffer must contain objects of the same underlying type. Each entry will also have a fixed number of *registers* that contain metadata relating to the current node. These registers will usually be interpreted as graph edges.

```
#[repr(C)]
#[derive(Copy, Clone)]
pub struct NodeAllocator<
    T: Default + Copy + Clone + Pod + Zeroable,
    const MAX_SIZE: usize,
    const NUM_REGISTERS: usize,
> {
    /// Size of the allocator
    pub size: u64,
    /// Furthest index of the allocator
    bump_index: u32,
    /// Buffer index of the first element in the free list
    free_list_head: u32,
    /// The first node of the is always reserved for the sentinel node
    /// so the max capacity is really `MAX_SIZE - 1` 
    pub nodes: [Node<NUM_REGISTERS, T>; MAX_SIZE],
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct Node<T: Copy + Clone + Pod + Zeroable + Default, const NUM_REGISTERS: usize> {
    /// Arbitrary registers (generally used for pointers)
    /// Note: Register 0 is ALWAYS used for the free list
    registers: [u32; NUM_REGISTERS],
    value: T,
}
```

The templated `NodeAllocator` object is flexible primitive data structure for implementing more complex types. Here's how one might use the `NodeAllocator` to implement a doubly-linked list:

```
// Register aliases
pub const PREV: u32 = 0;
pub const NEXT: u32 = 1;

#[derive(Copy, Clone)]
pub struct DLL<T: Default + Copy + Clone + Pod + Zeroable, const MAX_SIZE: usize> {
    pub head: u32,
    pub tail: u32,
    allocator: NodeAllocator<T, MAX_SIZE, 2>,
}
```

The DLL is essentially just a node allocator with 2 registers per node. These registers represent the `prev` and `next` pointers of a DLL node. The logic for how edges are created and removed are specific to the type, but the allocator struct provides an interface for implementing arbitrary types that have this property (trees and graphs).
