# Sokoban

Compact, efficient data structures in contiguous byte arrays.

### Benchmarks

Based on simple benchmarks, the naive performance of Sokoban data structures are on par with, but slightly slower than, the Rust Standard Library.

```
test bench_tests::bench_sokoban_avl_tree_insert_1000_u128             ... bench:     134,301 ns/iter (+/- 4,033)
test bench_tests::bench_sokoban_avl_tree_insert_1000_u128_stack       ... bench:     134,135 ns/iter (+/- 3,620)
test bench_tests::bench_sokoban_avl_tree_insert_20000_u128            ... bench:   2,744,853 ns/iter (+/- 158,364)
test bench_tests::bench_sokoban_avl_tree_remove_u128                  ... bench:     355,992 ns/iter (+/- 22,770)
test bench_tests::bench_sokoban_critbit_insert_1000_u128              ... bench:      90,306 ns/iter (+/- 590)
test bench_tests::bench_sokoban_critbit_insert_1000_u128_stack        ... bench:      76,819 ns/iter (+/- 661)
test bench_tests::bench_sokoban_critbit_insert_20000_u128             ... bench:   2,839,050 ns/iter (+/- 207,241)
test bench_tests::bench_sokoban_critbit_remove_1000_u128              ... bench:      97,366 ns/iter (+/- 6,124)
test bench_tests::bench_sokoban_hash_map_insert_1000_u128             ... bench:      46,828 ns/iter (+/- 1,928)
test bench_tests::bench_sokoban_hash_map_insert_1000_u128_stack       ... bench:      46,686 ns/iter (+/- 1,691)
test bench_tests::bench_sokoban_hash_map_insert_20000_u128            ... bench:   1,492,742 ns/iter (+/- 43,362)
test bench_tests::bench_sokoban_hash_map_remove_1000_u128             ... bench:      59,896 ns/iter (+/- 1,782)
test bench_tests::bench_sokoban_red_black_tree_insert_1000_u128       ... bench:      69,574 ns/iter (+/- 8,581)
test bench_tests::bench_sokoban_red_black_tree_insert_1000_u128_stack ... bench:      66,057 ns/iter (+/- 8,853)
test bench_tests::bench_sokoban_red_black_tree_insert_20000_u128      ... bench:   1,905,406 ns/iter (+/- 25,546)
test bench_tests::bench_sokoban_red_black_tree_remove_1000_u128       ... bench:     128,889 ns/iter (+/- 13,508)
test bench_tests::bench_std_btree_map_insert_1000_u128                ... bench:      51,353 ns/iter (+/- 10,240)
test bench_tests::bench_std_btree_map_insert_20000_u128               ... bench:   1,535,224 ns/iter (+/- 21,645)
test bench_tests::bench_std_btree_map_remove_1000_u128                ... bench:     131,879 ns/iter (+/- 19,325)
test bench_tests::bench_std_hash_map_insert_1000_u128                 ... bench:      38,775 ns/iter (+/- 237)
test bench_tests::bench_std_hash_map_insert_20000_u128                ... bench:     797,904 ns/iter (+/- 10,719)
test bench_tests::bench_std_hash_map_remove_1000_u128                 ... bench:      57,452 ns/iter (+/- 364)
```

### Why compact data structures?

For most applications, there is no reason to look past the Rust standard library for data structures. However, when the application has limited or expensive memory and is bottlenecked by performance, programmers will often need to design custom solutions to address those constraints. These types of constraints come up quite frequently in high frequency trading, embedded systems, and blockchain development.

Enter Sokoban: A library of data structures designed to simplify this exact problem.

### Generic Node Allocator

Almost all data structures can be represented by some sort of connected graph of nodes and edges. The `node-allocator` module implements a raw node allocation data structure for contiguous buffers. Each entry in the buffer must contain objects of the same underlying type. Each entry will also have a fixed number of _registers_ that contain metadata relating to the current node. These registers will usually be interpreted as graph edges.

```rust
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

```rust
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
