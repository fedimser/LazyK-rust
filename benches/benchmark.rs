use criterion::{criterion_group, criterion_main, Criterion};
use lazyk_rust::LazyKProgram;

fn parse_and_run(source: &str, input: &str) -> () {
    let mut program = LazyKProgram::compile(source).unwrap();
    program.run_string(input).unwrap();
}

fn criterion_benchmark(c: &mut Criterion) {
    let source = include_str!("../examples/reverse.lazy");
    let input = "abcde12345".repeat(100);
    c.bench_function("reverse 1000", |b| {
        b.iter(|| parse_and_run(&source, &input))
    });
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
