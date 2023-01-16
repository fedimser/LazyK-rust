#![allow(warnings, unused)] //TODO:REMOVE

use byteorder::{ReadBytesExt, WriteBytesExt};
/// Language spec: http://tromp.github.io/cl/lazy-k.html.
use std::{
    any::Any,
    collections::VecDeque,
    io::{stdin, stdout, BufRead, Cursor, ErrorKind, Stdin, Write},
    mem::take,
    ops::{Deref, DerefMut},
    rc::Rc,
};

type ExprId = usize;
pub enum Expr {
    A(ExprId, ExprId),
    K,
    K1(ExprId),
    S,
    S1(ExprId),
    S2(ExprId, ExprId),
    I,
    I1(ExprId),
    LazyRead(),
    Inc,
    Num(usize),
    Free,
}

impl Expr {
    /*fn print(&self, out: &mut String) {
        match self {
            Expr::A(arg1, arg2) => {
                out.push('(');
                arg1.deref().print(out);
                out.push(' ');
                arg2.deref().print(out);
                out.push(')');
            }
            Expr::K => out.push('K'),
            Expr::K1(arg1) => {
                out.push_str("[K ");
                arg1.print(out);
                out.push(']');
            }
            Expr::S => out.push('S'),
            Expr::S1(arg1) => {
                out.push_str("[S ");
                arg1.print(out);
                out.push(']');
            }
            Expr::S2(arg1, arg2) => {
                out.push_str("[S ");
                arg1.print(out);
                out.push(' ');
                arg2.print(out);
                out.push(']');
            }
            Expr::I => out.push('I'),
            Expr::I1(arg1) => {
                out.push_str(".");
                arg1.print(out);
            }
            Expr::LazyRead(_) => out.push_str("LazyRead"),
            Expr::Inc => out.push_str("Inc"),
            Expr::Num(num) => {
                out.push_str(&format!("{num}"));
            }
            Expr::Free => out.push_str("?"), // TODO: remove?
        }
    }

    fn debug_string(&self) -> String {
        let mut ans = String::new();
        self.print(&mut ans);
        return ans;
    }*/
}

pub struct ExpressionPool {
    e: Vec<Expr>,
    church_chars: Vec<ExprId>,
    S: ExprId,
    K: ExprId,
    I: ExprId,
    KI: ExprId,
    Inc: ExprId,
    Zero: ExprId,
    Iota: ExprId,
    input: Input,
}

impl ExpressionPool {
    fn new() -> Self {
        let mut pool: Vec<Expr> = Vec::with_capacity(10000000);
        let mut n = |expr: Expr| {
            pool.push(expr);
            return pool.len() - 1;
        };

        let K = n(Expr::K);
        let S = n(Expr::S);
        let I = n(Expr::I);
        let KI = n(Expr::K1(I));
        let SI = n(Expr::S1(I));
        let KS = n(Expr::K1(S));
        let KK = n(Expr::K1(K));
        let SKSK = n(Expr::S2(KS, K));
        let SIKS = n(Expr::S2(I, KS));
        let Iota = n(Expr::S2(SIKS, KK));
        let Inc = n(Expr::Inc);
        let Zero = n(Expr::Num(0));

        let mut church_chars = vec![KI, I];
        for i in 2..EOF_MARKER + 1 {
            church_chars.push(n(Expr::S2(SKSK, church_chars[i - 1])));
        }
        Self {
            e: pool,
            church_chars,
            S,
            K,
            I,
            KI,
            Inc,
            Zero,
            Iota,
            input: Input::Null,
        }
    }

    fn new_expr(&mut self, expr: Expr) -> ExprId {
        self.e.push(expr);
        return self.e.len() - 1;
    }

    fn partial_eval_primitive_application(&mut self, expr_id: ExprId) {
        match self.e[expr_id] {
            Expr::A(lhs, rhs) => {
                self.e[expr_id] = self.partial_eval_primitive_application_2(lhs, rhs);
            }
            _ => panic!("Not an application!"),
        }
    }

    // TODO: pull out?
    fn partial_eval_primitive_application_2(&mut self, lhs: ExprId, rhs: ExprId) -> Expr {
        let rhs = self.drop_i1(rhs);
        match &self.e[lhs] {
            Expr::K => Expr::K1(rhs),
            Expr::K1(arg1) => Expr::I1(*arg1),
            Expr::S => Expr::S1(rhs),
            Expr::I => Expr::I1(rhs),
            Expr::S1(arg1) => Expr::S2(*arg1, rhs),
            Expr::LazyRead() => {
                self.apply_lazy_read(  lhs, rhs)
            }
            Expr::S2(arg1, arg2) => {
                self.apply_s2(*arg1, *arg2, rhs)
            }
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
    fn apply_lazy_read(&mut self, lhs: ExprId, rhs: ExprId) -> Expr{
        let next_char = match self.input.read_byte() {
            Some(ch) => ch as usize,
            None => EOF_MARKER,
        };
        let ch = self.church_char(next_char);
        let x_rhs = self.new_expr(Expr::K1(ch));
        let x = self.new_expr(Expr::S2(self.I, x_rhs));
        let new_lazy_read = self.new_expr(Expr::LazyRead());
        let y = self.new_expr(Expr::K1(new_lazy_read));
        self.e[lhs] = Expr::S2(x, y);
        return self.partial_eval_primitive_application_2(lhs, rhs); // "fall through".
    }
    
    fn apply_s2(&mut self, arg1: ExprId, arg2:ExprId, rhs:ExprId) -> Expr{
        let new_lhs = self.partial_apply(arg1, rhs);
        Expr::A(new_lhs, self.partial_apply(arg2, rhs))
    }

    fn partial_apply(&mut self, lhs: ExprId, rhs: ExprId) -> ExprId {
        self.new_expr(Expr::A(lhs, rhs))
    }

    // TODO: skip extra clones.
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
            while let Expr::A(cur_arg1, _) = self.e[cur] {
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

    // TODO: this can be inlined.
    fn swap_arg1(&mut self, app_id: ExprId, other_arg1: &mut ExprId) {
        match &mut self.e[app_id] {
            Expr::A(arg1, _) => std::mem::swap(arg1, other_arg1),
            _ => panic!("Unexpected expression type."),
        }
    }

    fn church2int(&mut self, church: ExprId) -> Result<usize> {
        let inc = self.partial_apply(church, self.Inc);
        let e = self.partial_apply(inc, self.Zero);
        let result_id = self.partial_eval(e);
        match self.e[result_id] {
            Expr::Num(num) => Ok(num),
            _ => Err(Error::ResultIsNotChurchNumeral),
        }
    }

    fn car(&mut self, list: ExprId) -> ExprId {
        return self.partial_apply(list, self.K);
    }

    fn cdr(&mut self, list: ExprId) -> ExprId {
        return self.partial_apply(list, self.KI);
    }

    fn church_char(&self, mut idx: usize) -> ExprId {
        if idx > EOF_MARKER {
            idx = EOF_MARKER;
        }
        self.church_chars[idx]
    }

    fn run(&mut self, expr_id: ExprId, input: Input, output: &mut Output) -> Result<usize> {
        self.input = input;
        let lr = self.new_expr(Expr::LazyRead());
        let mut e = self.partial_apply(expr_id, lr);
        loop {
            let head = self.car(e);
            let ch = self.church2int(head)?;
            if ch >= EOF_MARKER {
                return Ok(ch - EOF_MARKER);
            }
            output.write_char(ch as u8)?;
            e = self.cdr(e);
        }
    }
}

pub struct ExprRef {
    pool: &'static ExpressionPool,
    id: ExprId,
}

///////////////////////////////////////////////////////////////////////////////

///////////////////////////////////////////////////////////////////////////////
/// I/O

#[derive(Debug)]
pub enum Error {
    ParseError,
    ResultIsNotChurchNumeral, //"Runtime error: invalid output format (result was not a number)"
    IoError,
    BufferLimitExceeded,
}
pub type Result<T> = std::result::Result<T, Error>;

static EOF_MARKER: usize = 256;
static EOF_CHAR: char = '\x05';

pub enum Input {
    Null,
    Reader(Box<dyn BufRead + 'static>),
}
impl Input {
    // TODO: return std::io::Result<()>.
    fn read_byte(&mut self) -> Option<u8> {
        match self {
            Input::Null => None,
            Input::Reader(reader) => match reader.as_mut().read_u8() {
                Ok(ch) => Some(ch),
                Err(err) if err.kind() == ErrorKind::UnexpectedEof => None,
                Err(err) => panic!("I/O error: {}", err),
            },
        }
    }

    fn read_ascii_char(&mut self) -> char {
        match self.read_byte() {
            Some(byte) => byte as char,
            None => EOF_CHAR,
        }
    }
}

pub enum Output {
    Null,
    Buffer(Vec<u8>, Option<usize>),
    Writer(Box<dyn Write + 'static>),
}
impl Output {
    // TODO: return std::io::Result<()>.
    fn write_char(&mut self, c: u8) -> Result<()> {
        match self {
            Self::Null => Err(Error::IoError),
            Self::Buffer(buf, None) => {
                buf.push(c);
                Ok(())
            }
            Self::Buffer(buf, Some(limit)) => {
                if buf.len() >= *limit {
                    return Err(Error::BufferLimitExceeded);
                }
                buf.push(c);
                Ok(())
            }
            Self::Writer(w) => {
               w.write_u8(c).map_err(|_| Error::IoError)
            }
        }
    }
}
impl Default for Output {
    fn default() -> Self {
        Self::Null
    }
}

///////////////////////////////////////////////////////////////////////////////

pub struct Parser {
    pool: &'static mut ExpressionPool,
}

impl Parser {
    //fn compose(f: ExprRef, g: ExprRef) -> ExprRef {
    //    ExprRef::new(Expr::S2(ExprRef::new(Expr::K1(f)), g))
    //}

    fn parse_jot(source: &mut &[u8], pool: &mut ExpressionPool) -> ExprId {
        let mut e = pool.I;
        let mut i = 0;
        while i != source.len() {
            if source[i] == ('0' as u8) {
                let lhs = pool.partial_apply(e, pool.S);
                e = pool.partial_apply(lhs, pool.K);
            } else if source[i] == ('1' as u8) {
                let rhs = pool.partial_apply(pool.K, e);
                e = pool.partial_apply(pool.S, rhs);
            }
            i += 1;
        }
        *source = &source[i..];
        return e;
    }

    fn skip_whitespace_and_comments(source: &mut &[u8]) {
        let mut is_comment = false;
        let mut i = 0;
        for i in 0..source.len() {
            if source[i] >= 128 {
                continue;
            }
            let ch = source[i] as char;
            if ch == '#' {
                is_comment = true;
            }
            if ch == '\n' {
                is_comment = false;
            }
            if ch <= ' ' || is_comment {
                continue;
            }
            *source = &source[i..];
            return;
        }
        *source = &source[source.len()..];
    }

    fn parse_expr(source: &mut &[u8], i_is_iota: bool, pool: &mut ExpressionPool) -> ExprId {
        Self::skip_whitespace_and_comments(source);
        if source.is_empty() {
            panic!("Unexpected end of source.")
        }
        let ch = source[0] as char;
        if ch == '0' || ch == '1' {
            return Self::parse_jot(source, pool);
        }

        *source = &source[1..];
        match ch {
            '`' | '*' => {
                let p = Self::parse_expr(source, ch == '*', pool);
                let q = Self::parse_expr(source, ch == '*', pool);
                pool.partial_apply(p, q)
            }
            '(' => Self::parse_manual_close(source, true, pool),
            ')' => panic!("Mismatched close-parenthesis!"),
            'k' | 'K' => pool.K,
            's' | 'S' => pool.S,
            'i' => {
                if (i_is_iota) {
                    pool.Iota
                } else {
                    pool.I
                }
            }
            'I' => pool.I,
            _ => panic!("Invalid character: [{}]", ch),
        }
    }

    fn parse_manual_close(
        source: &mut &[u8],
        expected_closing_paren: bool,
        pool: &mut ExpressionPool,
    ) -> ExprId {
        let mut e: Option<ExprId> = None;
        loop {
            Self::skip_whitespace_and_comments(source);
            if source.is_empty() || source[0] == (')' as u8) {
                break;
            }
            let e2 = Self::parse_expr(source, false, pool);
            e = match e {
                Some(e) => Some(pool.partial_apply(e, e2)),
                None => Some(e2),
            }
        }
        if expected_closing_paren {
            assert!(source[0] == (')' as u8), "Premature end of program!");
            *source = &source[1..];
        } else {
            assert!(source.is_empty(), "Unmatched trailing close-parenthesis!");
        }
        match e {
            Some(e) => e,
            None => pool.I,
        }
    }

    pub fn parse(source: &str, pool: &mut ExpressionPool) -> ExprId {
        let mut b = source.as_bytes();
        Self::parse_manual_close(&mut b, false, pool)
    }
}

/// Lazy-K interpreter.
pub struct LazyK {
    pool: ExpressionPool,
    input: Input,
    output: Output,
}

impl LazyK {
    pub fn new() -> Self {
        let mut pool = ExpressionPool::new();
        Self {
            pool: pool,
            input: Input::Null,
            output: Output::Null,
        }
    }

    pub fn run_vec(
        &mut self,
        program: ExprId,
        input: Vec<u8>,
        max_output: Option<usize>,
    ) -> Vec<u8> {
        let input = Input::Reader(Box::new(Cursor::new(input)));
        let mut output = Output::Buffer(Vec::new(), max_output);

        self.pool.run(program, input, &mut output);

        match output {
            Output::Buffer(result, _) => result,
            _ => panic!("TODO"),
        }
    }

    pub fn run_string_limited(
        &mut self,
        program: ExprId,
        input: &str,
        max_output: usize,
    ) -> String {
        let result = self.run_vec(program, input.as_bytes().to_owned(), Some(max_output));
        String::from_utf8_lossy(&result).to_string()
    }

    pub fn run_string(&mut self, program: ExprId, input: &str) -> String {
        let result = self.run_vec(program, input.as_bytes().to_owned(), None);
        String::from_utf8_lossy(&result).to_string()
    }

    pub fn run_console(&mut self, program: ExprId) {
        let input = Input::Reader(Box::new(stdin().lock()));
        let mut output = Output::Writer(Box::new(stdout().lock()));

        self.pool.run(program, input, &mut output);
    }

    pub fn parse(&mut self, source: &str) -> ExprId {
        Parser::parse(source, &mut self.pool)
    }
}

// Right now memory is used undafely, so tests don't work in parallel.
// Run with cargo test -- --test-threads=1
mod tests {
    use super::*;

    #[test]
    fn test_church2int() {
        let mut lk = LazyK::new();
        for i in 0..5 {
            assert_eq!(lk.pool.church2int(lk.pool.church_char(i)).unwrap(), i);
        }
    }

    #[test]
    fn test_identity() {
        let mut lk = LazyK::new();
        let program = lk.pool.I;
        assert_eq!(lk.run_string(program, ""), "");
        assert_eq!(lk.run_string(program, "a"), "a");
        assert_eq!(lk.run_string(program, "ab"), "ab");
        assert_eq!(lk.run_string(program, "abc"), "abc");
        assert_eq!(lk.run_string(program, "abcd"), "abcd");
    }

    #[test]
    fn test_hello_world() {
        let mut source = include_str!("../examples/hello_world.lazy");
        let mut lk = LazyK::new();
        let program = lk.parse(source);
        let result = lk.run_string(program, "");
        assert_eq!(result, "Hello, world!\n");
    }

    #[test]
    fn test_calc() {
        let mut source = include_str!("../examples/calc.lazy");
        let mut lk = LazyK::new();
        let program = lk.parse(source);

        assert_eq!(lk.run_string(program, "2+2"), "4\n");
        assert_eq!(lk.run_string(program, "3*4"), "12\n");
        assert_eq!(lk.run_string(program, "2+3*4"), "14\n");
        assert_eq!(lk.run_string(program, "(2+3)*4"), "20\n");
        assert_eq!(
            lk.run_string(program, "1000*1000*1000*1000*1000*1000"),
            "1000000000000000000\n"
        );
    }

    #[test]
    fn test_reverse() {
        let mut source = include_str!("../examples/reverse.lazy");

        let mut lk = LazyK::new();
        let program = lk.parse(source);

        assert_eq!(lk.run_string(program, "a"), "a");
        assert_eq!(lk.run_string(program, "ab"), "ba");
        assert_eq!(lk.run_string(program, "aba"), "aba");
        assert_eq!(lk.run_string(program, ""), "");
        assert_eq!(lk.run_string(program, "stressed"), "desserts");
        assert_eq!(lk.run_string(program, "Hello, world!"), "!dlrow ,olleH");
    }

    #[test]
    fn test_quine() {
        let source = include_str!("../examples/quine.lazy");
        let mut lk = LazyK::new();
        let program = lk.parse(source);
        assert_eq!(lk.run_string(program, "a"), source);
    }

    #[test]
    fn test_primes() {
        let source = include_str!("../examples/primes.lazy");
        let mut lk = LazyK::new();
        let program = lk.parse(source);
        assert_eq!(
            lk.run_string_limited(program, "", 70),
            "2 3 5 7 11 13 17 19 23 29 31 37 41 43 47 53 59 61 67 71 73 79 83 89 97"
        );
    }

    #[test]
    fn test_ab() {
        let source = include_str!("../examples/ab.lazy");
        let mut lk = LazyK::new();
        let program = lk.parse(source);
        assert_eq!(
            lk.run_string_limited(program, "", 20),
            "ABABABABABABABABABAB"
        );
    }
}
