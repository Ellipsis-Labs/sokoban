# Sokoban
Compact, efficient data structures in contiguous byte arrays.

## DISCLAIMER
This is unaudited code, and still very rough around the edges. Proceed with caution before using any of these data structures in a production system.

### Benchmarks
Based on simple benchmarks, the naive performance of Sokoban data structures are on par with, but slightly slower than, the Rust Standard Library. Critbit is the exception for small sizes, but this should not be surprising as the insertion is amortized O(1) with very few reorgs of the data.

```
test bench_tests::bench_sokoban_avl_tree_insert_1000_u128        ... bench:     197,978 ns/iter (+/- 14,744)
test bench_tests::bench_sokoban_avl_tree_insert_20000_u128       ... bench:   3,337,856 ns/iter (+/- 394,301)
test bench_tests::bench_sokoban_avl_tree_remove_u128             ... bench:     585,244 ns/iter (+/- 58,473)
test bench_tests::bench_sokoban_critbit_insert_1000_u128         ... bench:      12,067 ns/iter (+/- 1,158)
test bench_tests::bench_sokoban_critbit_insert_20000_u128        ... bench:     256,514 ns/iter (+/- 18,923)
test bench_tests::bench_sokoban_critbit_remove_1000_u128         ... bench:     116,700 ns/iter (+/- 7,000)
test bench_tests::bench_sokoban_hash_map_insert_1000_u128        ... bench:      32,192 ns/iter (+/- 2,592)
test bench_tests::bench_sokoban_hash_map_insert_20000_u128       ... bench:   1,134,509 ns/iter (+/- 119,435)
test bench_tests::bench_sokoban_hash_map_remove_1000_u128        ... bench:      48,177 ns/iter (+/- 6,349)
test bench_tests::bench_sokoban_red_black_tree_insert_1000_u128  ... bench:      45,940 ns/iter (+/- 3,609)
test bench_tests::bench_sokoban_red_black_tree_insert_20000_u128 ... bench:   1,716,138 ns/iter (+/- 157,458)
test bench_tests::bench_sokoban_red_black_tree_remove_1000_u128  ... bench:     118,634 ns/iter (+/- 9,840)
test bench_tests::bench_std_btree_map_insert_1000_u128           ... bench:      42,541 ns/iter (+/- 3,257)
test bench_tests::bench_std_btree_map_insert_20000_u128          ... bench:   1,095,174 ns/iter (+/- 140,250)
test bench_tests::bench_std_btree_map_remove_1000_u128           ... bench:     156,978 ns/iter (+/- 57,306)
test bench_tests::bench_std_hash_map_insert_1000_u128            ... bench:      23,454 ns/iter (+/- 2,287)
test bench_tests::bench_std_hash_map_insert_20000_u128           ... bench:     565,162 ns/iter (+/- 72,156)
test bench_tests::bench_std_hash_map_remove_1000_u128            ... bench:      44,043 ns/iter (+/- 2,788)
```


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
