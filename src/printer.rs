use crate::{
    expression::{Expr, ExprId},
    LazyKRunner, Style,
};

/// Prints LazyK program in prefix notation, substituting combinators
/// and application with custom strings.
pub(crate) struct GenericPrinter<'a> {
    runner: &'a LazyKRunner,
    a: &'static str,
    k: &'static str,
    s: &'static str,
    i: &'static str,
}

impl<'a> GenericPrinter<'a> {
    pub(crate) fn new(runner: &'a LazyKRunner, style: Style) -> Self {
        match style {
            Style::CombCalculus => panic!("Use CcPrinnter for Combinator-Calculus style."),
            Style::Unlambda => Self {
                runner,
                a: "`",
                k: "k",
                s: "s",
                i: "i",
            },
            Style::Jot => Self {
                runner,
                a: "1",
                k: "11100",
                s: "11111000",
                i: "11111111100000",
            },
            Style::Iota => Self {
                runner,

                a: "*",
                k: "*i*i*ii",
                s: "*i*i*i*ii",
                i: "*ii",
            },
        }
    }

    pub(crate) fn print(&self, expr_id: ExprId) -> String {
        let mut output = String::new();
        self.print_expr(expr_id, &mut output);
        output
    }

    fn print_expr(&'_ self, expr_id: ExprId, output: &mut String) {
        match self.runner.get_expr(expr_id) {
            Expr::A(arg1, arg2) => {
                output.push_str(self.a);
                self.print_expr(*arg1, output);
                self.print_expr(*arg2, output);
            }
            Expr::K => output.push_str(self.k),
            Expr::K1(arg) => {
                output.push_str(self.a);
                output.push_str(self.k);
                self.print_expr(*arg, output);
            }
            Expr::S => output.push_str(self.s),
            Expr::S1(arg) => {
                output.push_str(self.a);
                output.push_str(self.s);
                self.print_expr(*arg, output);
            }
            Expr::S2(arg1, arg2) => {
                output.push_str(self.a);
                output.push_str(self.a);
                output.push_str(self.s);
                self.print_expr(*arg1, output);
                self.print_expr(*arg2, output);
            }
            Expr::I => output.push_str(self.i),
            Expr::I1(arg) => {
                output.push_str(self.a);
                output.push_str(self.i);
                self.print_expr(*arg, output);
            }
            _ => panic!("Encountered unprintable expression type."),
        }
    }
}

/// Prints expression in combinator-calculus style.
pub(crate) struct CcPrinter<'a> {
    runner: &'a LazyKRunner,
}

impl<'a> CcPrinter<'a> {
    pub(crate) fn new(runner: &'a LazyKRunner) -> Self {
        Self { runner }
    }

    pub(crate) fn print(&self, expr_id: ExprId) -> String {
        let mut output = String::new();
        self.print_expr(expr_id, &mut output, false);
        output
    }

    fn print_expr(&self, expr_id: ExprId, output: &mut String, need_paren: bool) {
        match self.runner.get_expr(expr_id) {
            Expr::S => output.push('S'),
            Expr::K => output.push('K'),
            Expr::I => output.push('I'),
            expr => {
                if need_paren {
                    output.push('(');
                }
                match *expr {
                    Expr::A(arg1, arg2) => {
                        self.print_expr(arg1, output, false);
                        self.print_expr(arg2, output, true);
                    }
                    Expr::K1(arg) => {
                        output.push('K');
                        self.print_expr(arg, output, true);
                    }

                    Expr::S1(arg) => {
                        output.push('S');
                        self.print_expr(arg, output, true);
                    }
                    Expr::S2(arg1, arg2) => {
                        output.push('S');
                        self.print_expr(arg1, output, true);
                        self.print_expr(arg2, output, true);
                    }
                    Expr::I1(arg) => {
                        output.push('I');
                        self.print_expr(arg, output, true);
                    }
                    _ => panic!("Encountered unprintable expression type."),
                }
                if need_paren {
                    output.push(')');
                }
            }
        }
    }
}
