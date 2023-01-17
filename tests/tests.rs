use lazyk_rust::parser::{ExpressionPool, LazyKProgram};

#[test]
fn test_church2int() {
    let mut pool = ExpressionPool::new();
    for i in 0..5 {
        assert_eq!(pool.church2int(pool.church_char(i)).unwrap(), i);
    }
}

#[test]
fn test_identity() {
    let mut program = LazyKProgram::compile("I").unwrap();
    assert_eq!(program.run_string(""), "");
    assert_eq!(program.run_string("abcd"), "abcd");
}

#[test]
fn test_hello_world() {
    let source = include_str!("../examples/hello_world.lazy");
    let mut program = LazyKProgram::compile(source).unwrap();
    assert_eq!(program.run_string(""), "Hello, world!\n");
    assert_eq!(program.run_string("abcd"), "Hello, world!\n");
}

#[test]
fn test_calc() {
    let source = include_str!("../examples/calc.lazy");
    let mut program = LazyKProgram::compile(source).unwrap();

    assert_eq!(program.run_string("2+2"), "4\n");
    assert_eq!(program.run_string("3*4"), "12\n");
    assert_eq!(program.run_string("2+3*4"), "14\n");
    assert_eq!(program.run_string("(2+3)*4"), "20\n");
    assert_eq!(
        program.run_string("1000*1000*1000*1000*1000*1000"),
        "1000000000000000000\n"
    );
}

#[test]
fn test_reverse() {
    let source = include_str!("../examples/reverse.lazy");
    let mut program = LazyKProgram::compile(source).unwrap();

    assert_eq!(program.run_string("a"), "a");
    assert_eq!(program.run_string("ab"), "ba");
    assert_eq!(program.run_string("aba"), "aba");
    assert_eq!(program.run_string(""), "");
    assert_eq!(program.run_string("stressed"), "desserts");
    assert_eq!(program.run_string("Hello, world!"), "!dlrow ,olleH");
    assert_eq!(program.run_string("abcde12345".repeat(100).as_str()), "54321edcba".repeat(100));
}

#[test]
fn test_quine() {
    let source = include_str!("../examples/quine.lazy");
    let mut program = LazyKProgram::compile(source).unwrap();
    assert_eq!(program.run_string(""), source);
}

#[test]
fn test_primes() {
    let source = include_str!("../examples/primes.lazy");
    let mut program = LazyKProgram::compile(source).unwrap();
    assert_eq!(program.run_string_limited("", 1), "2");
    assert_eq!(
        program.run_string_limited("", 70),
        "2 3 5 7 11 13 17 19 23 29 31 37 41 43 47 53 59 61 67 71 73 79 83 89 97".replace(" ", "\n")
    );
}

#[test]
fn test_abab() {
    let source = include_str!("../examples/ab.lazy");
    let mut program = LazyKProgram::compile(source).unwrap();
    assert_eq!(program.run_string_limited("", 20), "ABABABABABABABABABAB");
}
