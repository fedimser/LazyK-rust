mod parser;
use std::env;
use std::fs;

fn main() {
    let args: Vec<String> = env::args().collect();
    assert!(args.len() >= 2);
    let source_path = &args[1];
    println!("source_path : {}", source_path);
    let source = match fs::read_to_string(source_path) {
        Ok(x) => x,
        Err(err) => panic!("Could not read source: {}", err),
    };
    let mut lazyk = parser::LazyK::new();
    let program = lazyk.parse(source.as_str());
    lazyk.run_console(program);
}
