disponent::declare!(
    #[disponent::configure(inherent)]
    pub enum Node {
        Leaf(Leaf),
        Branch(Branch),
    }

    pub trait Builder {
        fn duplicate(self) -> Self;
    }
);

pub struct Leaf;
pub struct Branch;

impl Builder for Leaf {
    fn duplicate(self) -> Self {
        Leaf
    }
}

impl Builder for Branch {
    fn duplicate(self) -> Self {
        Branch
    }
}

fn main() {
    let leaf = Node::Leaf(Leaf).duplicate();
    assert!(matches!(leaf, Node::Leaf(_)));

    let branch = Node::Branch(Branch).duplicate();
    assert!(matches!(branch, Node::Branch(_)));
}
