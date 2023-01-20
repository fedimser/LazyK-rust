use anyhow::Result;
use lazyk_rust::{LazyKProgram, LazyKRunner};
use std::ops::Deref;

#[test]
fn test_church2int() {
    let mut pool = LazyKRunner::new();
    for i in 0..5 {
        assert_eq!(pool.church2int(pool.church_char(i)).unwrap(), i);
    }
}

#[test]
fn test_identity() -> Result<()> {
    let source = "I";
    let mut program = LazyKProgram::compile(source).unwrap();
    assert_eq!(program.run_string("").unwrap(), "");
    assert_eq!(program.run_string("abcd")?, "abcd");
    Ok(())
}

#[test]
fn test_hello_world() -> Result<()> {
    let source = include_str!("../examples/hello_world.lazy");
    let mut program = LazyKProgram::compile(source).unwrap();
    assert_eq!(program.run_string("")?, "Hello, world!\n");
    assert_eq!(program.run_string("abcd")?, "Hello, world!\n");
    Ok(())
}

#[test]
fn test_calc() -> Result<()> {
    let source = include_str!("../examples/calc.lazy");
    let mut program = LazyKProgram::compile(source).unwrap();

    assert_eq!(program.run_string("2+2")?, "4\n");
    assert_eq!(program.run_string("3*4")?, "12\n");
    assert_eq!(program.run_string("2+3*4")?, "14\n");
    assert_eq!(program.run_string("(2+3)*4")?, "20\n");
    assert_eq!(
        program.run_string("1000*1000*1000*1000*1000*1000")?,
        "1000000000000000000\n"
    );
    Ok(())
}

#[test]
fn test_reverse() -> Result<()> {
    let source = include_str!("../examples/reverse.lazy");
    let mut program = LazyKProgram::compile(source).unwrap();
    assert_eq!(program.run_string("a")?, "a");
    assert_eq!(program.run_string("ab")?, "ba");
    assert_eq!(program.run_string("aba")?, "aba");
    assert_eq!(program.run_string("")?, "");
    assert_eq!(program.run_string("stressed")?, "desserts");
    assert_eq!(program.run_string("Hello, world!")?, "!dlrow ,olleH");
    assert_eq!(
        program.run_string("abcde12345".repeat(100).as_str())?,
        "54321edcba".repeat(100)
    );
    Ok(())
}

#[test]
fn test_quine() -> Result<()> {
    let source = include_str!("../examples/quine.lazy");
    let mut program = LazyKProgram::compile(source).unwrap();
    assert_eq!(program.run_string("")?, source);
    Ok(())
}

#[test]
fn test_primes() -> Result<()> {
    let source = include_str!("../examples/primes.lazy");
    let mut program = LazyKProgram::compile(source).unwrap();
    program.set_output_limit(Some(1));
    assert_eq!(program.run_string("")?, "2");
    program.set_output_limit(Some(70));
    assert_eq!(
        program.run_string("")?,
        "2 3 5 7 11 13 17 19 23 29 31 37 41 43 47 53 59 61 67 71 73 79 83 89 97".replace(" ", "\n")
    );
    Ok(())
}

#[test]
fn test_abab() -> Result<()> {
    let source = include_str!("../examples/ab.lazy");
    let mut program = LazyKProgram::compile(source).unwrap();
    program.set_output_limit(Some(100));
    assert_eq!(program.run_string("")?, "AB".repeat(50));
    Ok(())
}

fn assert_error<T>(x: Result<T>, expected_message: &str) {
    match x {
        Ok(_) => panic!("Expected error, got Ok."),
        Err(err) => assert_eq!(err.root_cause().deref().to_string(), expected_message),
    }
}

#[test]
fn test_parse_errors() -> Result<()> {
    assert_error(LazyKProgram::compile("((("), "Premature end of program.");
    assert_error(LazyKProgram::compile("abcd"), "Invalid character: [a]");
    assert_error(
        LazyKProgram::compile("(KS))"),
        "Unmatched trailing close-parenthesis.",
    );
    Ok(())
}

#[test]
fn test_runtime_errors() -> Result<()> {
    let mut program = LazyKProgram::compile("KSKSKSKKS")?;
    assert_error(
        program.run_string(""),
        "Program's output is not a church numeral.",
    );
    Ok(())
}

#[test]
fn test_make_printer() -> Result<()> {
    let text = "Hallo Welt!\n";
    let mut program = LazyKProgram::make_printer(text.as_bytes());
    assert_eq!(program.run_string("")?, "Hallo Welt!\n");

    let source = program.to_string();
    let expected_source = include_str!("../examples/hallo_welt.lazy");
    assert_eq!(source, expected_source);
    let mut program2 = LazyKProgram::compile(&source)?;
    assert_eq!(program2.run_string("")?, "Hallo Welt!\n");

    Ok(())
}
