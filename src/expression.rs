pub type ExprId = u32;

pub enum Expr {
    A(ExprId, ExprId),
    K,
    K1(ExprId),
    S,
    S1(ExprId),
    S2(ExprId, ExprId),
    I,
    I1(ExprId),
    LazyRead,
    Inc,
    Num(u16),
    Free,
}

#[test]
fn check_size() {
    assert_eq!(std::mem::size_of::<Expr>(), 12);
}
