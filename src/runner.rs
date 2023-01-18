use crate::{
    expression::{Expr, ExprId},
    io::{Input, Output},
};
use anyhow::{bail, Result};
use std::collections::VecDeque;

pub struct LazyKRunner {
    e: Vec<Expr>,
    free_ids: VecDeque<ExprId>,
    church_chars: Vec<ExprId>,
    pub s: ExprId,
    pub k: ExprId,
    pub i: ExprId,
    pub ki: ExprId,
    pub inc: ExprId,
    pub zero: ExprId,
    pub iota: ExprId,
    input: Input,
}

// Garbade collection is triggered if number of expressions in the pool exceeds
// this number.
static GC_LIMIT: usize = 1000000;
// Number of expressions at the beginning that are never garbage-collected.
static PREAMBLE_LENGTH: usize = 270;
// This Church number is used to mark end of input/output.
static EOF_MARKER: usize = 256;

impl LazyKRunner {
    pub fn new() -> Self {
        let mut pool: Vec<Expr> = Vec::with_capacity(GC_LIMIT);
        pool.push(Expr::Free);
        let mut n = |expr: Expr| {
            pool.push(expr);
            return pool.len() - 1;
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
        for i in 2..EOF_MARKER + 1 {
            church_chars.push(n(Expr::S2(sksk, church_chars[i - 1])));
        }
        assert!(pool.len() <= PREAMBLE_LENGTH);
        Self {
            e: pool,
            free_ids: VecDeque::new(),
            church_chars,
            s,
            k,
            i,
            ki,
            inc,
            zero,
            iota,
            input: Input::Null,
        }
    }

    fn new_expr(&mut self, expr: Expr) -> ExprId {
        if let Some(id) = self.free_ids.pop_front() {
            self.e[id] = expr;
            return id;
        } else {
            self.e.push(expr);
            return self.e.len() - 1;
        }
    }

    // Frees all expressions not reachable from expr_id.
    fn garbage_collect(&mut self, expr_id: ExprId) {
        if self.e.len() < GC_LIMIT || self.free_ids.len() > 0 {
            return;
        }

        let n = self.e.len();

        // BFS.
        let mut needed: Vec<bool> = vec![false; n];
        let mut queue: VecDeque<ExprId> = VecDeque::new();
        queue.push_back(expr_id);
        while let Some(next_id) = queue.pop_front() {
            if needed[next_id] {
                continue;
            }
            needed[next_id] = true;
            match &self.e[next_id] {
                Expr::A(arg1, arg2) | Expr::S2(arg1, arg2) => {
                    queue.push_back(*arg1);
                    queue.push_back(*arg2);
                }
                Expr::K1(arg1) | Expr::S1(arg1) | Expr::I1(arg1) => queue.push_back(*arg1),
                _ => {}
            }
        }
        for i in PREAMBLE_LENGTH..n {
            if needed[i] {
                continue;
            }
            match self.e[i] {
                Expr::Free => {}
                _ => {
                    self.e[i] = Expr::Free;
                    self.free_ids.push_back(i);
                }
            }
        }
    }

    fn partial_eval_primitive_application(&mut self, expr_id: ExprId) {
        match self.e[expr_id] {
            Expr::A(lhs, rhs) => {
                self.e[expr_id] = self.partial_eval_primitive_application_2(lhs, rhs);
            }
            _ => panic!("Not an application!"),
        }
    }

    fn partial_eval_primitive_application_2(&mut self, lhs: ExprId, rhs: ExprId) -> Expr {
        let rhs = self.drop_i1(rhs);
        match &self.e[lhs] {
            Expr::K => Expr::K1(rhs),
            Expr::K1(arg1) => Expr::I1(*arg1),
            Expr::S => Expr::S1(rhs),
            Expr::I => Expr::I1(rhs),
            Expr::S1(arg1) => Expr::S2(*arg1, rhs),
            Expr::LazyRead => self.apply_lazy_read(lhs, rhs),
            Expr::S2(arg1, arg2) => self.apply_s2(*arg1, *arg2, rhs),
            Expr::Inc => {
                let rhs2 = self.partial_eval(rhs);
                match self.e[rhs2] {
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
            Some(ch) => ch as usize,
            None => EOF_MARKER,
        };
        let ch = self.church_char(next_char);
        let x_rhs = self.new_expr(Expr::K1(ch));
        let x = self.new_expr(Expr::S2(self.i, x_rhs));
        let new_lazy_read = self.new_expr(Expr::LazyRead);
        let y = self.new_expr(Expr::K1(new_lazy_read));
        self.e[lhs] = Expr::S2(x, y);
        return self.partial_eval_primitive_application_2(lhs, rhs); // "fall through".
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
            if let Expr::I1(arg1) = self.e[cur] {
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
            while let Expr::A(_, _) = self.e[cur] {
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
        match &mut self.e[app_id] {
            Expr::A(arg1, _) => std::mem::swap(arg1, other_arg1),
            _ => panic!("Unexpected expression type."),
        }
    }

    pub fn church2int(&mut self, church: ExprId) -> Result<usize> {
        let inc = self.partial_apply(church, self.inc);
        let e = self.partial_apply(inc, self.zero);
        let result_id = self.partial_eval(e);
        match self.e[result_id] {
            Expr::Num(num) => Ok(num),
            _ => bail!("Program's output is not a church numeral."),
        }
    }

    fn car(&mut self, list: ExprId) -> ExprId {
        return self.partial_apply(list, self.k);
    }

    fn cdr(&mut self, list: ExprId) -> ExprId {
        return self.partial_apply(list, self.ki);
    }

    pub fn church_char(&self, mut idx: usize) -> ExprId {
        if idx > EOF_MARKER {
            idx = EOF_MARKER;
        }
        self.church_chars[idx]
    }

    pub fn run(
        &mut self,
        expr_id: ExprId,
        input: Input,
        output: &mut Output,
        output_limit: Option<usize>,
    ) -> Result<usize> {
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
            if output_size % 100 == 0 {
                self.garbage_collect(e);
            }
            if output_limit.is_some() && output_limit.unwrap() == output_size {
                return Ok(1);
            }
        }
    }
}
