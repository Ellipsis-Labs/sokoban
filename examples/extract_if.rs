use sokoban::{NodeAllocatorMap, RedBlackTree};

fn main() {
    {
        let mut tree = RedBlackTree::<u32, u32, 8>::new();
        tree.insert(0, 5); // this
        tree.insert(1, 5);
        tree.insert(2, 0); // this
        tree.insert(3, 5);
        tree.insert(4, 0); // this
        tree.insert(5, 5);
        tree.insert(6, 5);
        tree.insert(7, 0); // this

        println!("initial elements:");
        for x in tree.iter() {
            println!("initial node({}) {} {}", tree.get_addr(&x.0), x.0, x.1);
        }

        println!("\n Removing nodes");
        for x in tree.extract_if(my_predicate) {
            println!("removed node {} {}", x.0, x.1);
        }

        println!("\n remaining elements:");
        for x in tree.iter() {
            println!("remaining node({}) {} {}", tree.get_addr(&x.0), x.0, x.1);
        }
    }
}

fn my_predicate(key: &u32, value: &u32) -> bool {
    (*key == 0) | (*value == 0)
}
