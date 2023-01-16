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

pub enum Expr {
    A(ExprRef, ExprRef),
    K,
    K1(ExprRef),
    S,
    S1(ExprRef),
    S2(ExprRef, ExprRef),
    I,
    I1(ExprRef),
    LazyRead,
    Inc,
    Num(usize),
    Free,
}

impl Expr {
    fn print(&self, out: &mut String) {
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
            Expr::LazyRead => out.push_str("LazyRead"),
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
    }
}

static mut EXPRESSION_POOL: Vec<Expr> = Vec::new();
static NULL_ID: usize = 1000000000;
pub struct ExprRef(usize);

impl Deref for ExprRef {
    type Target = Expr;

    fn deref(&self) -> &Expr {
        assert!(self.0 != NULL_ID);
        unsafe { &EXPRESSION_POOL[self.0] }
    }
}

// TODO: garbage collection, or just replace with Rc<Expr>.
impl Clone for ExprRef {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl ExprRef {
    fn null() -> Self {
        ExprRef(NULL_ID)
    }

    fn is_null(&self) -> bool {
        return self.0 == NULL_ID;
    }

    fn new(expr: Expr) -> Self {
        unsafe {
            EXPRESSION_POOL.push(expr);
            if (EXPRESSION_POOL.len() % 1000000 == 0) {
                println!("Expressions in pool: {}", EXPRESSION_POOL.len());
            }
            return ExprRef(EXPRESSION_POOL.len() - 1);
        }
    }

    fn replace_with(&self, expr: Expr) {
        assert!(self.0 != NULL_ID);
        unsafe {
            EXPRESSION_POOL[self.0] = expr;
        }
    }

    fn swap_arg1(&self, other_arg1: &mut ExprRef) {
        assert!(self.0 != NULL_ID);
        let expr: &mut Expr = unsafe { &mut EXPRESSION_POOL[self.0] };
        match expr {
            Expr::A(arg1, _) => std::mem::swap(arg1, other_arg1),
            _ => panic!("Unexpected expression type."),
        }
    }
}

///////////////////////////////////////////////////////////////////////////////

struct Consts {
    church_chars: Vec<ExprRef>,
    S: ExprRef,
    K: ExprRef,
    Inc: ExprRef,
    Zero: ExprRef,
    Iota: ExprRef,
}

impl Consts {
    fn new() -> Self {
        unsafe {
            EXPRESSION_POOL.clear();
            EXPRESSION_POOL.reserve(10000000);
        }
        let K = ExprRef::new(Expr::K);
        let S = ExprRef::new(Expr::S);
        let I = ExprRef::new(Expr::I);
        let KI = ExprRef::new(Expr::K1(I.clone()));
        let SI = ExprRef::new(Expr::S1(I.clone()));
        let KS = ExprRef::new(Expr::K1(S.clone()));
        let KK = ExprRef::new(Expr::K1(K.clone()));
        let SKSK = ExprRef::new(Expr::S2(KS.clone(), K.clone()));
        let SIKS = ExprRef::new(Expr::S2(I.clone(), KS.clone()));
        let Iota = ExprRef::new(Expr::S2(SIKS, KK));
        let Inc = ExprRef::new(Expr::Inc);
        let Zero = ExprRef::new(Expr::Num(0));

        let mut church_chars = vec![KI, I];
        for i in 2..EOF_MARKER + 1 {
            church_chars.push(ExprRef::new(Expr::S2(
                SKSK.clone(),
                church_chars[i - 1].clone(),
            )));
        }
        Self {
            church_chars,
            S,
            K,
            Inc,
            Zero,
            Iota,
        }
    }

    fn church_char(&self, mut idx: usize) -> ExprRef {
        if idx > EOF_MARKER {
            idx = EOF_MARKER;
        }
        self.church_chars[idx].clone()
    }

    fn I(&self) -> ExprRef {
        self.church_chars[1].clone()
    }

    fn KI(&self) -> ExprRef {
        self.church_chars[0].clone()
    }

    fn K(&self) -> ExprRef {
        self.K.clone()
    }

    fn S(&self) -> ExprRef {
        self.S.clone()
    }

    fn Inc(&self) -> ExprRef {
        self.Inc.clone()
    }

    fn Zero(&self) -> ExprRef {
        self.Zero.clone()
    }

    fn Iota(&self) -> ExprRef {
        self.Iota.clone()
    }
}

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
            Self::Writer(w) => w.write_u8(c).map_err(|_| Error::IoError),
        }
    }
}
impl Default for Output {
    fn default() -> Self {
        Self::Null
    }
}

///////////////////////////////////////////////////////////////////////////////

/// Lazy-K interpreter.
pub struct LazyK {
    consts: Consts,
    input: Input,
    output: Output,
}

impl LazyK {
    pub fn new() -> Self {
        Self {
            consts: Consts::new(),
            input: Input::Null,
            output: Output::Null,
        }
    }

    fn partial_eval_primitive_application(&mut self, expr: &ExprRef) {
        match expr.deref() {
            Expr::A(lhs, rhs) => {
                expr.replace_with(self.partial_eval_primitive_application_2(lhs, rhs));
            }
            _ => panic!("Not an application!"),
        }
    }

    // TODO: pull out?
    fn partial_eval_primitive_application_2(&mut self, lhs: &ExprRef, rhs: &ExprRef) -> Expr {
        let rhs = Self::drop_i1(rhs.clone());
        match lhs.deref() {
            Expr::K => Expr::K1(rhs),
            Expr::K1(arg1) => Expr::I1(arg1.clone()),
            Expr::S => Expr::S1(rhs),
            Expr::I => Expr::I1(rhs),
            Expr::S1(arg1) => Expr::S2(arg1.clone(), rhs),
            Expr::LazyRead => {
                let next_char = match self.input.read_byte() {
                    Some(ch) => ch as usize,
                    None => EOF_MARKER,
                };
                let ch = self.consts.church_char(next_char);
                let x = ExprRef::new(Expr::S2(self.consts.I(), ExprRef::new(Expr::K1(ch))));
                let y = ExprRef::new(Expr::K1(ExprRef::new(Expr::LazyRead)));
                lhs.replace_with(Expr::S2(x, y));
                return self.partial_eval_primitive_application_2(lhs, &rhs); // "fall through".
            }
            Expr::S2(arg1, arg2) => Expr::A(
                Self::partial_apply(arg1.clone(), rhs.clone()),
                Self::partial_apply(arg2.clone(), rhs),
            ),
            Expr::Inc => {
                let rhs2 = self.partial_eval(rhs);
                match rhs2.deref() {
                    Expr::Num(num) => Expr::Num(num + 1),
                    _ => panic!("Attempted to apply inc to a non-number"),
                }
            }
            _ => panic!("Unreachable code."),
        }
    }

    fn partial_apply(lhs: ExprRef, rhs: ExprRef) -> ExprRef {
        ExprRef::new(Expr::A(lhs, rhs))
    }

    // TODO: skip extra clones.
    fn drop_i1(expr: ExprRef) -> ExprRef {
        let mut cur = expr;
        loop {
            if let Expr::I1(arg1) = cur.deref() {
                cur = arg1.clone();
            } else {
                return cur;
            }
        }
    }

    fn partial_eval(&mut self, mut cur: ExprRef) -> ExprRef {
        let mut prev = ExprRef::null();
        loop {
            cur = Self::drop_i1(cur);
            while let Expr::A(cur_arg1, _) = cur.deref() {
                cur.swap_arg1(&mut prev);
                let next = Self::drop_i1(prev);
                prev = cur;
                cur = next;
            }
            if prev.is_null() {
                return cur;
            }

            let mut next = cur;
            cur = prev;
            cur.swap_arg1(&mut next);
            prev = next;

            self.partial_eval_primitive_application(&cur);
        }
    }

    fn church2int(&mut self, church: ExprRef) -> Result<usize> {
        let e = Self::partial_apply(
            Self::partial_apply(church, self.consts.Inc()),
            self.consts.Zero(),
        );
        let result = self.partial_eval(e);
        match result.deref() {
            Expr::Num(num) => Ok(*num),
            _ => Err(Error::ResultIsNotChurchNumeral),
        }
    }

    fn car(&self, list: ExprRef) -> ExprRef {
        return Self::partial_apply(list, self.consts.K());
    }

    fn cdr(&self, list: ExprRef) -> ExprRef {
        return Self::partial_apply(list, self.consts.KI());
    }

    fn run(&mut self, program: ExprRef) -> Result<()> {
        let mut e = Self::partial_apply(program, ExprRef::new(Expr::LazyRead));
        loop {
            let ch = self.church2int(self.car(e.clone()))?;
            if ch >= EOF_MARKER {
                return Ok(());
            }
            self.output.write_char(ch as u8)?;
            e = self.cdr(e);
        }
    }

    pub fn run_vec(
        &mut self,
        program: &ExprRef,
        input: Vec<u8>,
        max_output: Option<usize>,
    ) -> Vec<u8> {
        self.input = Input::Reader(Box::new(Cursor::new(input)));
        self.output = Output::Buffer(Vec::new(), max_output);

        self.run(program.clone());

        self.input = Input::Null;
        match take(&mut self.output) {
            Output::Buffer(result, _) => result,
            _ => panic!("TODO"),
        }
    }

    pub fn run_string_limited(
        &mut self,
        program: &ExprRef,
        input: &str,
        max_output: usize,
    ) -> String {
        let result = self.run_vec(program, input.as_bytes().to_owned(), Some(max_output));
        String::from_utf8_lossy(&result).to_string()
    }

    pub fn run_string(&mut self, program: &ExprRef, input: &str) -> String {
        let result = self.run_vec(program, input.as_bytes().to_owned(), None);
        String::from_utf8_lossy(&result).to_string()
    }

    pub fn run_console(&mut self, program: ExprRef) {
        self.input = Input::Reader(Box::new(stdin().lock()));
        self.output = Output::Writer(Box::new(stdout().lock()));

        self.run(program);

        self.input = Input::Null;
        self.output = Output::Null;
    }

    fn compose(f: ExprRef, g: ExprRef) -> ExprRef {
        ExprRef::new(Expr::S2(ExprRef::new(Expr::K1(f)), g))
    }

    fn parse_jot(&self, source: &mut &[u8]) -> ExprRef {
        let mut e = self.consts.I();
        let mut i = 0;
        while i != source.len() {
            if source[i] == ('0' as u8) {
                e = Self::partial_apply(Self::partial_apply(e, self.consts.S()), self.consts.K());
            } else if source[i] == ('1' as u8) {
                e = Self::partial_apply(self.consts.S(), Self::partial_apply(self.consts.K(), e));
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

    fn parse_expr(&self, source: &mut &[u8], i_is_iota: bool) -> ExprRef {
        Self::skip_whitespace_and_comments(source);
        if source.is_empty() {
            panic!("Unexpected end of source.")
        }
        let ch = source[0] as char;
        if ch == '0' || ch == '1' {
            return self.parse_jot(source);
        }

        *source = &source[1..];
        match ch {
            '`' | '*' => {
                let p = self.parse_expr(source, ch == '*');
                let q = self.parse_expr(source, ch == '*');
                Self::partial_apply(p, q)
            }
            '(' => self.parse_manual_close(source, true),
            ')' => panic!("Mismatched close-parenthesis!"),
            'k' | 'K' => self.consts.K(),
            's' | 'S' => self.consts.S(),
            'i' => {
                if (i_is_iota) {
                    self.consts.Iota()
                } else {
                    self.consts.I()
                }
            }
            'I' => self.consts.I(),
            _ => panic!("Invalid character: [{}]", ch),
        }
    }

    fn parse_manual_close(&self, source: &mut &[u8], expected_closing_paren: bool) -> ExprRef {
        let mut e: Option<ExprRef> = None;
        loop {
            Self::skip_whitespace_and_comments(source);
            if source.is_empty() || source[0] == (')' as u8) {
                break;
            }
            let e2 = self.parse_expr(source, false);
            e = match e {
                Some(e) => Some(Self::partial_apply(e, e2)),
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
            None => self.consts.I(),
        }
    }

    pub fn parse(&mut self, source: &str) -> ExprRef {
        let mut b = source.as_bytes();
        self.parse_manual_close(&mut b, false)
    }
}

#[derive(PartialEq)]
enum Token {
    Char(char),
    JotExpr(String),
    Eof,
}

// Right now memory is used undafely, so tests don't work in parallel.
// Run with cargo test -- --test-threads=1
mod tests {
    use super::*;

    #[test]
    fn test_church2int() {
        let mut lk = LazyK::new();
        for i in 0..5 {
            assert_eq!(lk.church2int(lk.consts.church_char(i)).unwrap(), i);
        }
    }

    #[test]
    fn test_identity() {
        let mut lk = LazyK::new();
        let program = lk.consts.I();
        assert_eq!(lk.run_string(&program, ""), "");
        assert_eq!(lk.run_string(&program, "a"), "a");
        assert_eq!(lk.run_string(&program, "ab"), "ab");
        assert_eq!(lk.run_string(&program, "abc"), "abc");
        assert_eq!(lk.run_string(&program, "abcd"), "abcd");
    }

    #[test]
    fn test_hello_world() {
        let mut source = include_str!("../examples/hello_world.lazy");
        let mut lk = LazyK::new();
        let program = lk.parse(source);
        let result = lk.run_string(&program, "");
        assert_eq!(result, "Hello, world!\n");
    }

    #[test]
    fn test_calc() {
        let mut source = include_str!("../examples/calc.lazy");
        let mut lk = LazyK::new();
        let program = lk.parse(source);

        assert_eq!(lk.run_string(&program, "2+2"), "4\n");
        assert_eq!(lk.run_string(&program, "3*4"), "12\n");
        assert_eq!(lk.run_string(&program, "2+3*4"), "14\n");
        assert_eq!(lk.run_string(&program, "(2+3)*4"), "20\n");
        assert_eq!(
            lk.run_string(&program, "1000*1000*1000*1000*1000*1000"),
            "1000000000000000000\n"
        );
    }

    #[test]
    fn test_reverse() {
        let mut source = include_str!("../examples/reverse.lazy");

        let mut lk = LazyK::new();
        let program = lk.parse(source);

        assert_eq!(lk.run_string(&program, "a"), "a");
        assert_eq!(lk.run_string(&program, "ab"), "ba");
        assert_eq!(lk.run_string(&program, "aba"), "aba");
        assert_eq!(lk.run_string(&program, ""), "");
        assert_eq!(lk.run_string(&program, "stressed"), "desserts");
        assert_eq!(lk.run_string(&program, "Hello, world!"), "!dlrow ,olleH");
    }

    #[test]
    fn test_quine() {
        let source = include_str!("../examples/quine.lazy");
        let mut lk = LazyK::new();
        let program = lk.parse(source);
        assert_eq!(lk.run_string(&program, "a"), source);
    }

    #[test]
    fn test_primes() {
        let source = include_str!("../examples/primes.lazy");
        let mut lk = LazyK::new();
        let program = lk.parse(source);
        assert_eq!(
            lk.run_string_limited(&program, "", 70),
            "2 3 5 7 11 13 17 19 23 29 31 37 41 43 47 53 59 61 67 71 73 79 83 89 97"
        );
    }

    #[test]
    fn test_ab() {
        let source = include_str!("../examples/ab.lazy");
        let mut lk = LazyK::new();
        let program = lk.parse(source);
        assert_eq!(
            lk.run_string_limited(&program, "", 20),
            "ABABABABABABABABABAB"
        );
    }
}
