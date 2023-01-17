mod parser;
use std::env;
use std::fs;

use lazyk_rust::parser::LazyKProgram;

fn main() {
    let args: Vec<String> = env::args().collect();
    let mut source = "".to_string();
    if args.len() >= 2 {
        let source_path = &args[1];
        println!("source_path : {}", source_path);
        source = match fs::read_to_string(source_path) {
            Ok(x) => x,
            Err(err) => panic!("Could not read source: {}", err),
        };
    }
    let mut program = LazyKProgram::compile(&source).expect("Parse error.");
    match program.run_console() {
        Ok(_) => (),
        Err(_) => println!("Execution ended with error."),
    }
}
