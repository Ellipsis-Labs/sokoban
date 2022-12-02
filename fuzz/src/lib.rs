use arbitrary::Arbitrary;
use rand::thread_rng;
use rand::Rng;
use sokoban::NodeAllocatorMap;
use std::fmt::Debug;

#[derive(Debug, Arbitrary, Clone, Copy)]
pub enum NodeAllocatorMapAction<K: Copy, V: std::fmt::Debug + std::cmp::PartialEq + Copy> {
    Insert { key: K, value: V },
    Upsert { value: V },
    Remove,
    Replace { value: V },
    Iter,
    IterRev,
    IterMut,
    IterMutRev,
}

pub fn perform_action<K: Copy, V: std::fmt::Debug + std::cmp::PartialEq + Copy>(
    tree: &mut dyn NodeAllocatorMap<K, V>,
    keys: &mut Vec<K>,
    action: NodeAllocatorMapAction<K, V>,
) {
    let mut rng = thread_rng();
    match action {
        NodeAllocatorMapAction::Insert { key, value } => {
            if tree.get(&key).is_some() {
                return;
            }
            if tree.insert(key, value).is_some() {
                keys.push(key);
                assert_eq!(*tree.get(&key).unwrap(), value);
            }
        }
        NodeAllocatorMapAction::Upsert { value } => {
            if keys.len() == 0 {
                return;
            }
            let j = rng.gen_range(0, keys.len());
            let key = keys[j];
            tree.insert(key, value);
            assert_eq!(*tree.get(&key).unwrap(), value);
        }
        NodeAllocatorMapAction::Replace { value } => {
            if keys.len() == 0 {
                return;
            }
            let j = rng.gen_range(0, keys.len());
            let key = keys[j];
            *tree.get_mut(&key).unwrap() = value;
            assert_eq!(*tree.get(&key).unwrap(), value);
        }
        NodeAllocatorMapAction::Remove => {
            if keys.len() == 0 {
                return;
            }
            let j = rng.gen_range(0, keys.len());
            let key = keys[j];
            let value = *tree.get(&key).unwrap();
            assert_eq!(value, tree.remove(&key).unwrap());
            keys.swap_remove(j);
        }
        NodeAllocatorMapAction::Iter => {
            for (k, v) in tree.iter() {
                assert_eq!(*tree.get(k).unwrap(), *v);
            }
        }
        NodeAllocatorMapAction::IterRev => {
            for (k, v) in tree.iter().rev() {
                assert_eq!(*tree.get(k).unwrap(), *v);
            }
        }
        NodeAllocatorMapAction::IterMut => for (_k, _v) in tree.iter_mut() {},
        NodeAllocatorMapAction::IterMutRev => for (_k, _v) in tree.iter_mut().rev() {},
    }
}
