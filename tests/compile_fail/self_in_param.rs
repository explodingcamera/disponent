disponent::declare!(
    #[disponent::configure(inherent)]
    pub enum Holder {
        Num(NumHolder),
        Text(TextHolder),
    }

    pub trait HolderOps {
        fn take_other(&self, val: Self);
    }
);

pub struct NumHolder;
pub struct TextHolder;

impl HolderOps for NumHolder {
    fn take_other(&self, _val: Self) {}
}

impl HolderOps for TextHolder {
    fn take_other(&self, _val: Self) {}
}

fn main() {}
