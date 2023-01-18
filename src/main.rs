use clap::Parser;
use lazyk_rust::LazyKProgram;
use std::fs;

/// LazyK interpreter by Dmytro Fedoriaka.
#[derive(Parser, Debug)]
#[command(about)]
struct Args {
    /// Path to LazyK program to run.
    #[arg(index = 1)]
    program_file: String,

    /// Indicates that PROGRAM_FILE should be interpreted as in-line LazyK code.
    #[arg(short)]
    e: bool,
}

fn main() {
    let args = Args::parse();
    let source = if args.e {
        args.program_file
    } else {
        match fs::read_to_string(args.program_file) {
            Ok(x) => x,
            Err(err) => {
                println!("Could not read source: {}", err);
                return;
            }
        }
    };

    let mut program = match LazyKProgram::compile(&source) {
        Ok(program) => program,
        Err(err) => {
            println!("Parsing error: {}", err);
            return;
        }
    };

    match program.run_console() {
        Ok(_) => {}
        Err(err) => println!("Runtime error: {}", err),
    }
}
