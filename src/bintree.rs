#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BinTree<T> {
    Leaf(T),
    Node {
        left: Box<BinTree<T>>,
        right: Box<Option<BinTree<T>>>,
    },
}

impl<T: Clone> BinTree<T> {
    pub fn leaf(value: T) -> Self {
        Self::Leaf(value)
    }

    pub fn node(left: Self, right: Option<Self>) -> Self {
        Self::Node {
            left: Box::new(left),
            right: Box::new(right),
        }
    }

    pub fn is_leaf(&self) -> bool {
        matches!(self, Self::Leaf(_))
    }

    pub fn is_node(&self) -> bool {
        matches!(self, Self::Node { .. })
    }

    pub fn height(&self) -> usize {
        match self {
            Self::Leaf(_) => 1,
            Self::Node { left, right } => {
                1 + left.height().max(if let Some(node) = (**right).clone() {
                    node.height()
                } else {
                    0
                })
            }
        }
    }

    pub fn leaf_count(&self) -> usize {
        match self {
            Self::Leaf(_) => 1,
            Self::Node { left, right } => {
                left.leaf_count()
                    + if let Some(node) = (**right).clone() {
                        node.leaf_count()
                    } else {
                        0
                    }
            }
        }
    }

    pub fn from_vec(leaves: Vec<T>) -> Self {
        assert!(!leaves.is_empty(), "cannot build tree from empty vec");
        let nodes: Vec<BinTree<T>> = leaves.into_iter().map(|x| Self::leaf(x)).collect();
        Self::build_tree(nodes)
    }

    fn build_tree(nodes: Vec<BinTree<T>>) -> Self {
        assert!(!nodes.is_empty(), "cannot build tree from empty vec");
        if nodes.len() == 1 {
            nodes[0].clone()
        } else {
            let mut _nodes = Vec::new();
            let n = nodes.len();

            for i in (0..n).step_by(2) {
                let left = nodes[i].clone();
                let right = if i + 1 < n {
                    Some(nodes[i + 1].clone())
                } else {
                    None
                };
                _nodes.push(Self::node(left, right));
            }
            Self::build_tree(_nodes)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    // Collect leaf values left-to-right (preorder-ish but stable for comparing multisets)
    fn collect_leaves<T: Copy>(t: &BinTree<T>, out: &mut Vec<T>) {
        match t {
            BinTree::Leaf(v) => out.push(*v),
            BinTree::Node { left, right } => {
                collect_leaves(left, out);
                if let Some(node) = (**right).clone() {
                    collect_leaves(&node, out);
                }
            }
        }
    }

    // -------------------------
    // Unit tests
    // -------------------------

    #[test]
    #[should_panic(expected = "cannot build tree from empty vec")]
    fn from_vec_panics_on_empty() {
        let _t: BinTree<u32> = BinTree::from_vec(vec![]);
    }

    #[test]
    fn from_vec_single_leaf_is_leaf() {
        let t = BinTree::from_vec(vec![42u32]);
        match t {
            BinTree::Leaf(v) => assert_eq!(v, 42),
            BinTree::Node { .. } => panic!("expected Leaf"),
        }
        assert_eq!(t.leaf_count(), 1);
        assert_eq!(t.height(), 1);
    }

    #[test]
    fn from_vec_leaf_count_matches_input_len_small() {
        let t = BinTree::from_vec(vec![1u32, 2, 3, 4, 5]);
        assert_eq!(t.leaf_count(), 5);
    }

    #[test]
    fn from_vec_preserves_leaf_multiset_small() {
        let input = vec![5u32, 1, 5, 9, 1, 2];
        let t = BinTree::from_vec(input.clone());

        let mut got = Vec::new();
        collect_leaves(&t, &mut got);

        let mut a = input;
        let mut b = got;
        a.sort_unstable();
        b.sort_unstable();
        assert_eq!(a, b);
    }

    // For powers of two, this construction is perfectly balanced:
    // height = log2(n) + 1 (counting leaf level as height 1).
    #[test]
    fn from_vec_power_of_two_height_is_log2_plus_one() {
        for &n in &[1usize, 2, 4, 8, 16, 32] {
            let input: Vec<u32> = (0..n as u32).collect();
            let t = BinTree::from_vec(input);
            let expected = n.trailing_zeros() as usize + 1;
            assert_eq!(t.height(), expected);
            assert_eq!(t.leaf_count(), n);
        }
    }

    // -------------------------
    // Property-based tests
    // -------------------------

    proptest! {
        // Property 1: leaf_count(tree) == leaves.len() for any non-empty input.
        #[test]
        fn prop_leaf_count_matches_len(xs in proptest::collection::vec(any::<u32>(), 1..512)) {
            let n = xs.len();
            let t = BinTree::from_vec(xs);
            prop_assert_eq!(t.leaf_count(), n);
        }

        // Property 2: leaf multiset is preserved (handles duplicates by sorting).
        #[test]
        fn prop_leaf_multiset_preserved(xs in proptest::collection::vec(any::<u32>(), 1..512)) {
            let t = BinTree::from_vec(xs.clone());

            let mut got = Vec::new();
            collect_leaves(&t, &mut got);

            let mut a = xs;
            let mut b = got;
            a.sort_unstable();
            b.sort_unstable();

            prop_assert_eq!(a, b);
        }

        // Property 3: height bounds are sane: 1 <= height <= n
        // (loose but always true and catches certain structural bugs).
        #[test]
        fn prop_height_in_bounds(xs in proptest::collection::vec(any::<u32>(), 1..512)) {
            let n = xs.len();
            let t = BinTree::from_vec(xs);
            let h = t.height();
            prop_assert!(h >= 1);
            prop_assert!(h <= n);
        }

        // Property 4: for power-of-two lengths, height == log2(n) + 1.
        // We generate n as 2^k.
        #[test]
        fn prop_power_of_two_height_exact(k in 0u8..10) {
            let n = 1usize << k; // 1..512
            let input: Vec<u32> = (0..n as u32).collect();
            let t = BinTree::from_vec(input);
            let expected = k as usize + 1;
            prop_assert_eq!(t.height(), expected);
            prop_assert_eq!(t.leaf_count(), n);
        }
    }
}
