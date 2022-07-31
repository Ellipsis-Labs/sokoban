use bytemuck::Pod;
use bytemuck::Zeroable;
use critbit::*;
use rand::rngs::ThreadRng;
use rand::thread_rng;
use rand::{self, Rng};
use std::collections::BTreeMap;

const MAX_SIZE: usize = 20001;
const NUM_NODES: usize = 2 * MAX_SIZE;

#[repr(C)]
#[derive(Default, Copy, Clone, PartialEq)]
struct Widget {
    a: u128,
    b: u128,
    size: u64,
}

unsafe impl Zeroable for Widget {}
unsafe impl Pod for Widget {}

impl Widget {
    pub fn new_random(r: &mut ThreadRng) -> Self {
        Self {
            a: r.gen::<u128>(),
            b: r.gen::<u128>(),
            size: r.gen::<u64>(),
        }
    }
}

#[tokio::test(threaded_scheduler)]
async fn test_simulate() {
    type CritbitTree = Critbit<Widget, NUM_NODES, MAX_SIZE>;
    let mut buf = vec![0u8; std::mem::size_of::<CritbitTree>()];
    let cbt = CritbitTree::new_from_slice(buf.as_mut_slice());
    println!("Critbit Memory Size: {}", std::mem::size_of::<CritbitTree>());
    println!("Critbit Capacity: {}", MAX_SIZE - 1);
    let mut rng = thread_rng();
    let mut keys = vec![];
    let mut s = 0;
    let mut map = BTreeMap::new();
    for _ in 0..(MAX_SIZE - 1) {
        let k = rng.gen::<u128>();
        let v = Widget::new_random(&mut rng);
        if cbt.insert(k, v) == None {
            assert!(false)
        }
        s += 1;
        map.insert(k, v);
        keys.push(k);
    }

    for k in keys.iter() {
        if cbt.remove(*k) == None {
            assert!(false)
        }
        s -= 1;
        map.remove(k);
    }
    keys = vec![];

    for _i in 0..(MAX_SIZE - 1) {
        let k = rng.gen::<u128>();
        let v = Widget::new_random(&mut rng);
        if cbt.insert(k, v) == None {
            assert!(false)
        }
        s += 1;
        map.insert(k, v);
        keys.push(k);
    }

    for _ in 0..100000 {
        assert!(s == cbt.size());
        let sample = rng.gen::<f64>();
        if sample < 0.33 {
            if cbt.size() >= MAX_SIZE - 1 {
                continue;
            }
            let k = rng.gen::<u128>();
            let v = Widget::new_random(&mut rng);
            if cbt.insert(k, v) == None {
                assert!(false);
            }
            s += 1;
            map.insert(k, v);
            keys.push(k);
        } else if sample < 0.66 {
            if keys.is_empty() {
                continue;
            }
            let j = rng.gen_range(0, keys.len());
            let key = keys[j];
            keys.swap_remove(j);
            cbt.remove(key);
            map.remove(&key);
            s -= 1;
        } else {
            if keys.is_empty() {
                continue;
            }
            let j = rng.gen_range(0, keys.len());
            let key = keys[j];
            let v = Widget::new_random(&mut rng);
            cbt.insert(key, v);
            map.insert(key, v);
        }
    }

    let leaves = cbt.inorder_traversal();
    for ((k1, v1), (k2, v2)) in map.iter().zip(leaves.iter()) {
        assert!(*k1 == *k2);
        assert!(*v1 == *v2);
    }
}
