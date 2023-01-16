use criterion::{black_box, criterion_group, criterion_main, Criterion};
use lazyk_rust::parser::LazyK;

fn parse_and_run(source: &str, input: &str) -> () {
    let mut lk = LazyK::new();
    let program = lk.parse(source);
    lk.run_string(program, input);
}

fn criterion_benchmark(c: &mut Criterion) {
    let mut source = include_str!("../examples/reverse.lazy");    
    let input = "abcde12345".repeat(100);

    c.bench_function("reverse 1000", |b| b.iter(|| parse_and_run(&source, &input)));
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
