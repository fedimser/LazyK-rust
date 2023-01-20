use assert_cmd::Command;
use predicates::str::contains;

#[test]
fn called_with_no_args() {
    Command::cargo_bin("lazyk-rust")
        .unwrap()
        .assert()
        .failure()
        .stderr(contains("Usage: lazyk-rust <PROGRAM_FILE>"));
}

#[test]
fn prints_help() {
    Command::cargo_bin("lazyk-rust")
        .unwrap()
        .arg("-h")
        .assert()
        .success()
        .stdout(contains("LazyK interpreter"));
}

#[test]
fn executes_inline_program() {
    Command::cargo_bin("lazyk-rust")
        .unwrap()
        .args(["-e", include_str!("../examples/reverse.lazy")])
        .write_stdin("abcd")
        .assert()
        .success()
        .stdout("dcba");
}

#[test]
fn parse_error() {
    Command::cargo_bin("lazyk-rust")
        .unwrap()
        .args(["-e", "SK("])
        .assert()
        .success()
        .stdout("Parsing error: Premature end of program.\n");
}

#[test]
fn runtime_error() {
    Command::cargo_bin("lazyk-rust")
        .unwrap()
        .args(["-e", "KK"])
        .write_stdin("a")
        .assert()
        .success()
        .stdout("Runtime error: Program\'s output is not a church numeral.\n");
}

#[test]
fn executes_program_from_file() {
    Command::cargo_bin("lazyk-rust")
        .unwrap()
        .args(["./examples/hello_world.lazy"])
        .assert()
        .success()
        .stdout("Hello, world!\n");
}
