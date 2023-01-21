use crate::{
    expression::{Expr, ExprId},
    io::{Input, Output},
    util::{num_repr, NumRepr},
};
use anyhow::{bail, Result};
use std::{collections::VecDeque, mem::size_of};

pub struct LazyKRunner {
    e: Vec<Expr>,
    church_chars: Vec<ExprId>,
    pub s: ExprId,
    pub k: ExprId,
    pub i: ExprId,
    pub ki: ExprId,
    pub inc: ExprId,
    pub zero: ExprId,
    pub iota: ExprId,
    input: Input,

    // If zero, there are no Free slots.
    // If not zero, id of next potential slot.
    gc_free_ptr: usize,
    gc_queue: VecDeque<ExprId>,
}

// Garbade collector will try to keep memory usage below this number.
static GC_LIMIT_BYTES: usize = 256 * 1024 * 1024;
static GC_LIMIT_EXPR: usize = GC_LIMIT_BYTES / size_of::<Expr>();
// Number of expressions at the beginning that are never garbage-collected.
static PREAMBLE_LENGTH: usize = 448;
// This Church number is used to mark end of input/output.
static EOF_MARKER: u16 = 256;

impl LazyKRunner {
    pub fn new() -> Self {
        let mut pool: Vec<Expr> = Vec::with_capacity(1000000);
        pool.push(Expr::Free);
        let mut n = |expr: Expr| {
            pool.push(expr);
            (pool.len() - 1) as ExprId
        };

        let k = n(Expr::K);
        let s = n(Expr::S);
        let i = n(Expr::I);
        let ki = n(Expr::K1(i));
        let ks = n(Expr::K1(s));
        let kk = n(Expr::K1(k));
        let sksk = n(Expr::S2(ks, k));
        let siks = n(Expr::S2(i, ks));
        let iota = n(Expr::S2(siks, kk));
        let inc = n(Expr::Inc);
        let zero = n(Expr::Num(0));

        let mut church_chars = vec![ki, i];
        for i in 2..=EOF_MARKER {
            let church_expr = match num_repr(i) {
                NumRepr::Pow(a, b) => Expr::A(church_chars[b], church_chars[a]),
                NumRepr::Mul(a, b) => Expr::S2(n(Expr::K1(church_chars[a])), church_chars[b]),
                NumRepr::Inc(a) => Expr::S2(sksk, church_chars[a]),
            };
            church_chars.push(n(church_expr));
        }
        assert!(pool.len() <= PREAMBLE_LENGTH);
        Self {
            e: pool,
            church_chars,
            s,
            k,
            i,
            ki,
            inc,
            zero,
            iota,
            input: Input::Null,
            gc_free_ptr: 0,
            gc_queue: VecDeque::new(),
        }
    }

    #[inline(always)]
    fn new_expr_push(&mut self, expr: Expr) -> ExprId {
        let ans = self.e.len() as ExprId;
        self.e.push(expr);
        ans
    }

    pub(crate) fn new_expr(&mut self, expr: Expr) -> ExprId {
        if self.gc_free_ptr == 0 {
            return self.new_expr_push(expr);
        }
        // Try use next free slot.
        for i in self.gc_free_ptr..self.e.len() {
            if let Expr::Free = self.e[i] {
                self.e[i] = expr;
                self.gc_free_ptr = i + 1;
                return i as ExprId;
            }
        }
        // Reached end of allocated pool, push.
        self.gc_free_ptr = 0;
        self.new_expr_push(expr)
    }

    // Frees all expressions not reachable from expr_id.
    fn garbage_collect(&mut self, expr_id: ExprId) {
        let fp = self.gc_free_ptr;
        let n = self.e.len();
        let need_gc = (fp == 0 && n > GC_LIMIT_EXPR) || (fp > GC_LIMIT_EXPR);
        if !need_gc {
            return;
        }

        // BFS.
        let mut needed: Vec<bool> = vec![false; n];
        self.gc_queue.push_back(expr_id);
        while let Some(next_id) = self.gc_queue.pop_front() {
            if needed[next_id as usize] {
                continue;
            }
            needed[next_id as usize] = true;
            match &self.e[next_id as usize] {
                Expr::A(arg1, arg2) | Expr::S2(arg1, arg2) => {
                    self.gc_queue.push_back(*arg1);
                    self.gc_queue.push_back(*arg2);
                }
                Expr::K1(arg1) | Expr::S1(arg1) | Expr::I1(arg1) => self.gc_queue.push_back(*arg1),
                _ => {}
            }
        }
        #[allow(clippy::needless_range_loop)]
        for i in PREAMBLE_LENGTH..n {
            if !needed[i] {
                self.e[i] = Expr::Free;
            }
        }
        self.gc_free_ptr = PREAMBLE_LENGTH;
    }

    fn partial_eval_primitive_application(&mut self, expr_id: ExprId) {
        match self.e[expr_id as usize] {
            Expr::A(lhs, rhs) => {
                self.e[expr_id as usize] = self.partial_eval_primitive_application_2(lhs, rhs);
            }
            _ => panic!("Not an application!"),
        }
    }

    fn partial_eval_primitive_application_2(&mut self, lhs: ExprId, rhs: ExprId) -> Expr {
        let rhs = self.drop_i1(rhs);
        match &self.e[lhs as usize] {
            Expr::K => Expr::K1(rhs),
            Expr::K1(arg1) => Expr::I1(*arg1),
            Expr::S => Expr::S1(rhs),
            Expr::I => Expr::I1(rhs),
            Expr::S1(arg1) => Expr::S2(*arg1, rhs),
            Expr::LazyRead => self.apply_lazy_read(lhs, rhs),
            Expr::S2(arg1, arg2) => self.apply_s2(*arg1, *arg2, rhs),
            Expr::Inc => {
                let rhs2 = self.partial_eval(rhs);
                match self.e[rhs2 as usize] {
                    Expr::Num(num) => Expr::Num(num + 1),
                    _ => panic!("Attempted to apply inc to a non-number"),
                }
            }
            _ => panic!("Unreachable code."),
        }
    }

    // lhs points to LazyRead.
    fn apply_lazy_read(&mut self, lhs: ExprId, rhs: ExprId) -> Expr {
        let next_char = match self.input.read_byte() {
            Some(ch) => ch as u16,
            None => EOF_MARKER,
        };
        let ch = self.church_char(next_char);
        let x_rhs = self.new_expr(Expr::K1(ch));
        let x = self.new_expr(Expr::S2(self.i, x_rhs));
        let new_lazy_read = self.new_expr(Expr::LazyRead);
        let y = self.new_expr(Expr::K1(new_lazy_read));
        self.e[lhs as usize] = Expr::S2(x, y);
        self.partial_eval_primitive_application_2(lhs, rhs)
    }

    fn apply_s2(&mut self, arg1: ExprId, arg2: ExprId, rhs: ExprId) -> Expr {
        let new_lhs = self.partial_apply(arg1, rhs);
        Expr::A(new_lhs, self.partial_apply(arg2, rhs))
    }

    pub fn partial_apply(&mut self, lhs: ExprId, rhs: ExprId) -> ExprId {
        self.new_expr(Expr::A(lhs, rhs))
    }

    fn drop_i1(&self, expr: ExprId) -> ExprId {
        let mut cur = expr;
        loop {
            if let Expr::I1(arg1) = self.e[cur as usize] {
                cur = arg1;
            } else {
                return cur;
            }
        }
    }

    fn partial_eval(&mut self, mut cur: ExprId) -> ExprId {
        let mut prev: ExprId = 0;
        loop {
            cur = self.drop_i1(cur);
            while let Expr::A(_, _) = self.e[cur as usize] {
                self.swap_arg1(cur, &mut prev);
                let next = self.drop_i1(prev);
                prev = cur;
                cur = next;
            }
            if prev == 0 {
                return cur;
            }

            let mut next = cur;
            cur = prev;
            self.swap_arg1(cur, &mut next);
            prev = next;

            self.partial_eval_primitive_application(cur);
        }
    }

    fn swap_arg1(&mut self, app_id: ExprId, other_arg1: &mut ExprId) {
        match &mut self.e[app_id as usize] {
            Expr::A(arg1, _) => std::mem::swap(arg1, other_arg1),
            _ => panic!("Unexpected expression type."),
        }
    }

    pub fn church2int(&mut self, church: ExprId) -> Result<u16> {
        let inc = self.partial_apply(church, self.inc);
        let e = self.partial_apply(inc, self.zero);
        let result_id = self.partial_eval(e);
        match self.e[result_id as usize] {
            Expr::Num(num) => Ok(num),
            _ => bail!("Program's output is not a church numeral."),
        }
    }

    fn car(&mut self, list: ExprId) -> ExprId {
        self.partial_apply(list, self.k)
    }

    fn cdr(&mut self, list: ExprId) -> ExprId {
        self.partial_apply(list, self.ki)
    }

    // pair(X,Y)F := (FX)Y
    // pair(X,Y) = S(SI(KX))(KY)
    pub(crate) fn pair(&mut self, x: ExprId, y: ExprId) -> ExprId {
        let d = self.new_expr(Expr::K1(x));
        let a = self.new_expr(Expr::S2(self.i, d));
        let b = self.new_expr(Expr::K1(y));
        self.new_expr(Expr::S2(a, b))
    }

    pub fn church_char(&self, mut idx: u16) -> ExprId {
        if idx > EOF_MARKER {
            idx = EOF_MARKER;
        }
        self.church_chars[idx as usize]
    }

    pub fn run(
        &mut self,
        expr_id: ExprId,
        input: Input,
        output: &mut Output,
        output_limit: Option<usize>,
    ) -> Result<u16> {
        self.input = input;
        let lr = self.new_expr(Expr::LazyRead);
        let mut e = self.partial_apply(expr_id, lr);
        let mut output_size = 0;
        loop {
            let head = self.car(e);
            let ch = self.church2int(head)?;
            if ch >= EOF_MARKER {
                return Ok(ch - EOF_MARKER);
            }
            output.write_char(ch as u8)?;
            e = self.cdr(e);
            output_size += 1;
            self.garbage_collect(e);
            if output_limit.is_some() && output_limit.unwrap() == output_size {
                return Ok(1);
            }
        }
    }

    /// Prints expression in combinator-calculus style.
    pub(crate) fn print_expr_cc(&self, expr_id: ExprId, output: &mut String, need_paren: bool) {
        match self.e[expr_id as usize] {
            Expr::S => output.push('S'),
            Expr::K => output.push('K'),
            Expr::I => output.push('I'),
            ref expr => {
                if need_paren {
                    output.push('(');
                }
                match *expr {
                    Expr::A(arg1, arg2) => {
                        self.print_expr_cc(arg1, output, false);
                        self.print_expr_cc(arg2, output, true);
                    }
                    Expr::K1(arg) => {
                        output.push('K');
                        self.print_expr_cc(arg, output, true);
                    }

                    Expr::S1(arg) => {
                        output.push('S');
                        self.print_expr_cc(arg, output, true);
                    }
                    Expr::S2(arg1, arg2) => {
                        output.push('S');
                        self.print_expr_cc(arg1, output, true);
                        self.print_expr_cc(arg2, output, true);
                    }
                    Expr::I1(arg) => {
                        output.push('I');
                        self.print_expr_cc(arg, output, true);
                    }
                    _ => panic!("Encountered unprintable expression type."),
                }
                if need_paren {
                    output.push(')');
                }
            }
        }
    }

    /// Prints expression in Unlambda style.
    pub(crate) fn print_expr_ul(&self, expr_id: ExprId, output: &mut String) {
        match self.e[expr_id as usize] {
            Expr::A(arg1, arg2) => {
                output.push('`');
                self.print_expr_ul(arg1, output);
                self.print_expr_ul(arg2, output);
            }
            Expr::K => output.push('k'),
            Expr::K1(arg) => {
                output.push_str("`k");
                self.print_expr_ul(arg, output);
            }
            Expr::S => output.push('s'),
            Expr::S1(arg) => {
                output.push_str("`s");
                self.print_expr_ul(arg, output);
            }
            Expr::S2(arg1, arg2) => {
                output.push_str("``s");
                self.print_expr_ul(arg1, output);
                self.print_expr_ul(arg2, output);
            }
            Expr::I => output.push('i'),
            Expr::I1(arg) => {
                output.push_str("`i");
                self.print_expr_ul(arg, output);
            }
            _ => panic!("Encountered unprintable expression type."),
        }
    }
}

impl Default for LazyKRunner {
    fn default() -> Self {
        Self::new()
    }
}
