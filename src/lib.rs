use anyhow::Result;
use std::io::{stdin, stdout, Cursor};
use {
    expression::ExprId,
    io::{Input, Output},
    parser::Parser,
    runner::LazyKRunner,
};

pub mod expression;
pub mod io;
pub mod parser;
pub mod runner;

pub struct LazyKProgram {
    pool: LazyKRunner,
    root_id: ExprId,
    output_limit: Option<usize>,
}

/// Compiled LazyK program, ready to be executed.
/// 
/// # Example
///
/// ```
/// use lazyk_rust::LazyKProgram;
/// let source = "I";
/// let mut program = LazyKProgram::compile(source).unwrap();
/// assert_eq!(program.run_string("abcd").unwrap(), "abcd");
/// ```
impl LazyKProgram {
    // Compiles LazyK source to a runnable program.
    pub fn compile(source: &str) -> Result<Self> {
        let mut pool = LazyKRunner::new();
        let root_id = Parser::parse(source, &mut pool)?;
        Ok(Self {
            pool,
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
        self.pool
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
        self.pool
            .run(self.root_id, input, &mut output, self.output_limit)?;
        Ok(())
    }
}
