# Sokoban

Compact, efficient data structures in contiguous byte arrays.

### Benchmarks

Based on simple benchmarks, the naive performance of Sokoban data structures are on par with, but slightly slower than, the Rust Standard Library.

```
test bench_tests::bench_sokoban_avl_tree_insert_1000_u128             ... bench:     121,553 ns/iter (+/- 2,933)
test bench_tests::bench_sokoban_avl_tree_insert_1000_u128_stack       ... bench:     124,535 ns/iter (+/- 3,075)
test bench_tests::bench_sokoban_avl_tree_insert_20000_u128            ... bench:   2,582,045 ns/iter (+/- 120,419)
test bench_tests::bench_sokoban_avl_tree_lookup_20000_u128            ... bench:   1,107,122 ns/iter (+/- 308,951)
test bench_tests::bench_sokoban_avl_tree_remove_1000_u128             ... bench:       1,438 ns/iter (+/- 16)
test bench_tests::bench_sokoban_critbit_insert_1000_u128              ... bench:      72,208 ns/iter (+/- 1,030)
test bench_tests::bench_sokoban_critbit_insert_1000_u128_stack        ... bench:      72,413 ns/iter (+/- 1,746)
test bench_tests::bench_sokoban_critbit_insert_20000_u128             ... bench:   2,054,727 ns/iter (+/- 51,871)
test bench_tests::bench_sokoban_critbit_lookup_20000_u128             ... bench:   1,353,104 ns/iter (+/- 25,792)
test bench_tests::bench_sokoban_critbit_remove_1000_u128              ... bench:       1,437 ns/iter (+/- 53)
test bench_tests::bench_sokoban_hash_map_insert_1000_u128             ... bench:      43,634 ns/iter (+/- 3,473)
test bench_tests::bench_sokoban_hash_map_insert_1000_u128_stack       ... bench:      43,432 ns/iter (+/- 572)
test bench_tests::bench_sokoban_hash_map_insert_20000_u128            ... bench:   1,370,297 ns/iter (+/- 18,634)
test bench_tests::bench_sokoban_hash_map_lookup_20000_u128            ... bench:     721,154 ns/iter (+/- 22,961)
test bench_tests::bench_sokoban_hash_map_remove_1000_u128             ... bench:       8,340 ns/iter (+/- 253)
test bench_tests::bench_sokoban_hash_set_insert_1000_u128             ... bench:      30,949 ns/iter (+/- 339)
test bench_tests::bench_sokoban_hash_set_insert_1000_u128_stack       ... bench:      30,989 ns/iter (+/- 562)
test bench_tests::bench_sokoban_hash_set_insert_20000_u128            ... bench:     620,947 ns/iter (+/- 21,835)
test bench_tests::bench_sokoban_hash_set_lookup_20000_u128            ... bench:     460,327 ns/iter (+/- 5,141)
test bench_tests::bench_sokoban_hash_set_remove_1000_u128             ... bench:       9,646 ns/iter (+/- 83)
test bench_tests::bench_sokoban_red_black_tree_insert_1000_u128       ... bench:      62,692 ns/iter (+/- 5,134)
test bench_tests::bench_sokoban_red_black_tree_insert_1000_u128_stack ... bench:      62,022 ns/iter (+/- 10,104)
test bench_tests::bench_sokoban_red_black_tree_insert_20000_u128      ... bench:   1,757,891 ns/iter (+/- 88,025)
test bench_tests::bench_sokoban_red_black_tree_lookup_20000_u128      ... bench:   1,156,224 ns/iter (+/- 72,865)
test bench_tests::bench_sokoban_red_black_tree_remove_1000_u128       ... bench:       1,474 ns/iter (+/- 45)
test bench_tests::bench_std_btree_map_insert_1000_u128                ... bench:      49,594 ns/iter (+/- 1,474)
test bench_tests::bench_std_btree_map_insert_20000_u128               ... bench:   1,411,543 ns/iter (+/- 7,937)
test bench_tests::bench_std_btree_map_lookup_20000_u128               ... bench:     871,666 ns/iter (+/- 42,544)
test bench_tests::bench_std_btree_map_remove_1000_u128                ... bench:         865 ns/iter (+/- 9)
test bench_tests::bench_std_hash_map_insert_1000_u128                 ... bench:      37,456 ns/iter (+/- 488)
test bench_tests::bench_std_hash_map_insert_20000_u128                ... bench:     770,091 ns/iter (+/- 7,708)
test bench_tests::bench_std_hash_map_lookup_20000_u128                ... bench:     199,332 ns/iter (+/- 5,718)
test bench_tests::bench_std_hash_map_remove_1000_u128                 ... bench:       9,585 ns/iter (+/- 645)
test bench_tests::bench_std_hash_set_insert_1000_u128                 ... bench:      77,075 ns/iter (+/- 11,515)
test bench_tests::bench_std_hash_set_insert_20000_u128                ... bench:   1,514,386 ns/iter (+/- 414,434)
test bench_tests::bench_std_hash_set_lookup_20000_u128                ... bench:     195,316 ns/iter (+/- 1,390)
test bench_tests::bench_std_hash_set_remove_1000_u128                 ... bench:       8,837 ns/iter (+/- 127)
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
