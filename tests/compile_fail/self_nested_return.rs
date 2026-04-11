disponent::declare!(
    #[disponent::configure(inherent)]
    pub enum Holder {
        Num(NumHolder),
        Text(TextHolder),
    }

    pub trait HolderOps {
        fn pair(self) -> (i32, Self);
    }
);

pub struct NumHolder;
pub struct TextHolder;

impl HolderOps for NumHolder {
    fn pair(self) -> (i32, Self) {
        (1, NumHolder)
    }
}

impl HolderOps for TextHolder {
    fn pair(self) -> (i32, Self) {
        (2, TextHolder)
    }
}

fn main() {}
