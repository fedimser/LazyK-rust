use anyhow::Result;
use std::io::{stdin, stdout, Cursor};

use crate::{runner::LazyKRunner, expression::{ExprId, Expr}, parser::Parser, io::{Input, Output}};

pub struct LazyKProgram {
    runner: LazyKRunner,
    root_id: ExprId,
    output_limit: Option<usize>,
}

/// Compiled LazyK program, ready to be executed.
impl LazyKProgram {
    /// Compiles LazyK source to a runnable program.
    ///
    /// ```
    /// use lazyk_rust::LazyKProgram;
    /// let source = "I";
    /// let mut program = LazyKProgram::compile(source).unwrap();
    /// assert_eq!(program.run_string("abcd").unwrap(), "abcd");
    /// ```
    pub fn compile(source: &str) -> Result<Self> {
        let mut runner = LazyKRunner::new();
        let root_id = Parser::parse(source, &mut runner)?;
        Ok(Self {
            runner,
            root_id,
            output_limit: None,
        })
    }

    /// Sets maximal number of cbytes in output, after which program halts.
    /// Useful for running programs that produce infinite ouput.
    pub fn set_output_limit(&mut self, value: Option<usize>) {
        self.output_limit = value;
    }

    /// Runs program as Vec<u8> -> Vec<u8> function.
    pub fn run_vec(&mut self, input: Vec<u8>) -> Result<Vec<u8>> {
        let input = Input::Reader(Box::new(Cursor::new(input)));
        let mut output = Output::Buffer(Vec::new());
        self.runner
            .run(self.root_id, input, &mut output, self.output_limit)?;
        match output {
            Output::Buffer(result) => Ok(result),
            _ => panic!("Unreachable code."),
        }
    }

    /// Runs program as String -> String function.
    pub fn run_string(&mut self, input: &str) -> Result<String> {
        let result = self.run_vec(input.as_bytes().to_owned())?;
        Ok(String::from_utf8(result).map_err(anyhow::Error::from)?)
    }

    /// Runs program, reading from standard input and writing to standard output.
    pub fn run_console(&mut self) -> Result<()> {
        let input = Input::Reader(Box::new(stdin().lock()));
        let mut output = Output::Writer(Box::new(stdout().lock()));
        self.runner
            .run(self.root_id, input, &mut output, self.output_limit)?;
        Ok(())
    }

    /// Produces source code for this program (in combinator-caclulus style).
    ///
    /// ```
    /// use lazyk_rust::LazyKProgram;
    /// let prog = LazyKProgram::compile("```ssk``s`k``ss`s``sskk").unwrap();
    /// assert_eq!(prog.to_string(), "SSK(S(K(SS(S(SSK))))K)");
    /// ```
    pub fn to_string(&self) -> String {
        let mut output = String::new();
        self.runner.print_expr_cc(self.root_id, &mut output, false);
        output
    }

    /// Produces source code for this program (in Unlambda style).
    ///
    /// ```
    /// use lazyk_rust::LazyKProgram;
    /// let prog = LazyKProgram::compile("SSK(S(K(SS(S(SSK))))K)").unwrap();
    /// assert_eq!(prog.to_string_unlambda(), "```ssk``s`k``ss`s``sskk");
    /// ```
    pub fn to_string_unlambda(&self) -> String {
        let mut output = String::new();
        self.runner.print_expr_ul(self.root_id, &mut output);
        output
    }

    /// Produces LazyK program that prints given byte sequence to output.
    ///
    /// ```
    /// use lazyk_rust::LazyKProgram;
    /// let mut prog = LazyKProgram::make_printer("Hi!".as_bytes());
    /// assert_eq!(prog.run_string("").unwrap(), "Hi!");
    /// ```
    pub fn make_printer(bytes: &[u8]) -> LazyKProgram {
        let mut runner = LazyKRunner::new();
        let eof = runner.church_char(256);
        let mut list = runner.pair(eof, runner.k);
        for i in (0..bytes.len()).rev() {
            list = runner.pair(runner.church_char(bytes[i] as u16), list);
        }
        let root_id = runner.new_expr(Expr::K1(list));
        Self {
            runner,
            root_id,
            output_limit: None,
        }
    }
}
