use crate::known_deep_size;
use crate::DeepSizeOf;

use alloc::{boxed::Box, string::String, vec};
use core::mem::size_of;
use std::convert::TryInto;

#[test]
fn primitive_types() {
    assert_eq!(0u8.deep_size_of(), 1);
    assert_eq!(0u16.deep_size_of(), 2);
    assert_eq!(0u32.deep_size_of(), 4);
    assert_eq!(0u64.deep_size_of(), 8);
    assert_eq!(0usize.deep_size_of(), size_of::<usize>());

    assert_eq!(0i8.deep_size_of(), 1);
    assert_eq!(0i16.deep_size_of(), 2);
    assert_eq!(0i32.deep_size_of(), 4);
    assert_eq!(0i64.deep_size_of(), 8);
    assert_eq!(0isize.deep_size_of(), size_of::<isize>());

    assert_eq!(0f32.deep_size_of(), 4);
    assert_eq!(0f64.deep_size_of(), 8);

    assert_eq!('f'.deep_size_of(), 4);
    assert_eq!("Hello World!".deep_size_of(), 12);
    assert_eq!((&"Hello World!").deep_size_of(), 16);
    assert_eq!(true.deep_size_of(), 1);
}

#[test]
fn boxes() {
    let boxed = Box::new(0u32);
    assert_eq!(boxed.deep_size_of(), 4 + size_of::<usize>());
}

#[test]
fn arcs() {
    use std::sync::Arc;
    let test: Arc<[u32]> = vec![1, 2, 3].into();
    let multiple = (Arc::clone(&test), Arc::clone(&test), test);

    assert_eq!(
        multiple.deep_size_of(),
        3 * size_of::<Arc<[u32]>>() + 3 * size_of::<u32>()
    );
}

#[test]
fn slices() {
    let array: Box<[u32]> = vec![0; 64].into_boxed_slice();
    assert_eq!(array[5..10].deep_size_of(), 4 * 5);
    assert_eq!(array[..32].deep_size_of(), 4 * 32);
    assert_eq!(
        DeepSizeOf::deep_size_of(&array),
        size_of::<usize>() * 2 + size_of::<[u32; 64]>()
    );

    let array: Box<[u32; 1000]> = vec![0; 1000].into_boxed_slice().try_into().unwrap();

    assert_eq!(
        DeepSizeOf::deep_size_of(&array),
        size_of::<usize>() + size_of::<[u32; 1000]>()
    );
}

// TODO: find edge cases
#[test]
fn alignment() {
    #[repr(align(256))]
    struct Test(u8);
    known_deep_size!(0; Test);

    struct Test2(Test, u8);
    known_deep_size!(0; Test2);

    let array: [Test; 3] = [Test(5), Test(16), Test(2)];
    assert_eq!(size_of::<[Test; 3]>(), array.deep_size_of());

    let vec = vec![Test(5), Test(16), Test(2)];
    assert_eq!(vec.deep_size_of(), 256 * 3 + 24);

    let vec = vec![Test2(Test(5), 0), Test2(Test(16), 0), Test2(Test(2), 0)];
    assert_eq!(vec.deep_size_of(), 512 * 3 + 24);
}

#[test]
fn strings() {
    let string_a = String::from("01234567");
    let string_b = String::from("0123456789012345");

    assert_eq!(string_a.deep_size_of(), size_of::<String>() + 8);
    assert_eq!(string_b.deep_size_of(), size_of::<String>() + 16);
}

#[test]
fn tuples() {
    // Alignment - ######## #.##....
    let non_allocating = (45u64, (), (8u8, 16u16));
    let text = "Hello World";
    let allocating = (Box::new(42u32), String::from(text));

    assert_eq!(
        non_allocating.deep_size_of(),
        size_of::<(u64, (), (u8, u16))>()
    );
    assert_eq!(
        allocating.deep_size_of(),
        size_of::<(Box<()>, String)>() + text.len() + size_of::<u32>()
    );
}

mod context_tests {
    use crate::Context;

    #[test]
    fn context_arc_test() {
        let mut context = Context::new();

        let arc = alloc::sync::Arc::new(15);
        assert_eq!(context.contains_arc(&arc), false);
        context.add_arc(&arc);
        assert_eq!(context.contains_arc(&arc), true);
    }

    #[test]
    fn context_rc_test() {
        let mut context = Context::new();

        let rc = alloc::rc::Rc::new(15);
        assert_eq!(context.contains_rc(&rc), false);
        context.add_rc(&rc);
        assert_eq!(context.contains_rc(&rc), true);
    }
}

#[cfg(feature = "petgraph")]
#[test]
fn stable_digraph() {
    use petgraph::graph::{Edge, Node};
    use petgraph::stable_graph::StableDiGraph;

    let mut graph = StableDiGraph::<String, Box<u64>>::with_capacity(4, 4);
    let a = graph.add_node(String::with_capacity(11));
    let b = graph.add_node(String::with_capacity(17));
    let removed_node = graph.add_node(String::with_capacity(23));
    let live_edge = graph.add_edge(a, b, Box::new(5));
    let removed_edge = graph.add_edge(b, a, Box::new(8));

    assert_eq!(graph.remove_node(removed_node).unwrap().capacity(), 23);
    assert!(graph.remove_edge(removed_edge).is_some());

    let (node_capacity, edge_capacity) = graph.capacity();
    let live_node_capacity =
        graph.node_weight(a).unwrap().capacity() + graph.node_weight(b).unwrap().capacity();
    let expected = size_of::<StableDiGraph<String, Box<u64>>>()
        + node_capacity * size_of::<Node<Option<String>>>()
        + edge_capacity * size_of::<Edge<Option<Box<u64>>>>()
        + live_node_capacity
        + size_of::<u64>();

    assert_eq!(
        graph.edge_weight(live_edge).unwrap().deep_size_of(),
        2 * size_of::<u64>()
    );
    assert_eq!(graph.deep_size_of(), expected);
}

#[cfg(feature = "bimap")]
#[test]
fn bimap() {
    use alloc::rc::Rc;
    use bimap::BiMap;

    let mut map = BiMap::<String, Box<u64>>::with_capacity(4);
    map.insert(String::with_capacity(11), Box::new(5));
    map.insert(String::with_capacity(17), Box::new(8));

    let expected_children = map.iter().fold(0, |sum, (left, right)| {
        sum + left.deep_size_of() + right.deep_size_of()
    });
    let expected_maps = map.capacity() * 2 * (size_of::<Rc<String>>() + size_of::<Rc<Box<u64>>>());

    assert_eq!(
        map.deep_size_of(),
        size_of::<BiMap<String, Box<u64>>>() + expected_children + expected_maps
    );
}

#[cfg(feature = "derive")]
mod test_derive {
    use super::*;

    #[test]
    fn test_1() {
        #[derive(DeepSizeOf)]
        struct Example<'a>(&'a u32, &'a u32);

        let number = &42;
        let example = Example(number, number);

        let size = example.deep_size_of();
        // Data past references is not counted
        assert_eq!(size, 2 * size_of::<usize>());
    }

    #[test]
    fn test_enum() {
        #[derive(DeepSizeOf)]
        enum ExampleEnum {
            One,
            Two(),
            Three(u32, Box<u8>),
            Four { name: Box<u32> },
            Five {},
        }

        let variant_one = ExampleEnum::One;
        let variant_two = ExampleEnum::Two();
        let variant_three = ExampleEnum::Three(0, Box::new(255));
        let variant_four = ExampleEnum::Four {
            name: Box::new(65536),
        };
        let variant_five = ExampleEnum::Five {};

        assert_eq!(variant_one.deep_size_of(), size_of::<ExampleEnum>());
        assert_eq!(variant_two.deep_size_of(), size_of::<ExampleEnum>());
        assert_eq!(
            variant_three.deep_size_of(),
            size_of::<ExampleEnum>() + size_of::<u8>()
        );
        assert_eq!(
            variant_four.deep_size_of(),
            size_of::<ExampleEnum>() + size_of::<u32>()
        );
        assert_eq!(variant_five.deep_size_of(), size_of::<ExampleEnum>());
    }
}
