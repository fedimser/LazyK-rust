use anyhow::Result;
use std::io::{stdin, stdout, Cursor};

use crate::{
    expression::{Expr, ExprId},
    io::{Input, Output},
    parser::Parser,
    printer::{CcPrinter, GenericPrinter},
    runner::LazyKRunner,
};

pub enum Style {
    CombCalculus,
    Unlambda,
    Jot,
    Iota,
}

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
        String::from_utf8(result).map_err(anyhow::Error::from)
    }

    /// Runs program, reading from standard input and writing to standard output.
    pub fn run_console(&mut self) -> Result<()> {
        let input = Input::Reader(Box::new(stdin().lock()));
        let mut output = Output::Writer(Box::new(stdout().lock()));
        self.runner
            .run(self.root_id, input, &mut output, self.output_limit)?;
        Ok(())
    }

    /// Produces source code for this program.
    ///
    /// There are four supported styles: combinator-calculus, Unlambda, Jot and
    /// Iota.
    ///
    /// ```
    /// use lazyk_rust::{LazyKProgram, Style};
    /// let prog = LazyKProgram::compile("S(SI(K(KI)))(K(KI))").unwrap();
    /// assert_eq!(prog.to_source(Style::CombCalculus), "S(SI(K(KI)))(K(KI))");
    /// assert_eq!(prog.to_source(Style::Unlambda), "``s``si`k`ki`k`ki");                                         
    /// assert_eq!(prog.to_source(Style::Jot), "11111110001111111000111111111000001111001111001111111110000011110011110011111111100000");
    /// assert_eq!(prog.to_source(Style::Iota), "***i*i*i*ii***i*i*i*ii*ii**i*i*ii**i*i*ii*ii**i*i*ii**i*i*ii*ii");
    /// ```
    pub fn to_source(&self, style: Style) -> String {
        match style {
            Style::CombCalculus => CcPrinter::new(&self.runner).print(self.root_id),
            _ => GenericPrinter::new(&self.runner, style).print(self.root_id),
        }
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
