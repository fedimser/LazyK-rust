# LazyK-rust
Interpreter for Lazy K programming language, written in Rust.

Lazy K is an esoteric pure functional programming language designed by Ben Rudiak-Gould, based on the SKI combinator calculus. 


## Usage as binary

To get the binary, clone this repository and run `cargo build`.

The following command runs Lazy program from given source file, reading from the standard input and writing to the standard output:
```
lazyk-rust <path_to_source>
```

The following command runs Lazy program from given inline source, reading from the standard input and writing to the standard output:
```
lazyk-rust -e <source>
```

For example, `lazyk-rust -e I` runs the identity function, it copies input to output (until EOF is reached).

See specification below for details on how I/O works.

## Usage as library

Use the `LazyKProgram` class. For example: 

```
use lazyk_rust::LazyKProgram;
let source = "I";
let mut program = LazyKProgram::compile(source).unwrap();
assert_eq!(program.run_string("abcd").unwrap(), "abcd");
```

For more details, see tests and `LazyKProgram` class documentation.

## Implemenation details

This interpreter fully implements the specification. It's also fully safe (it doesn't use unsafe Rust).

It uses generally the same approach as in the reference implementation (`lazy_orig.cpp`), where all transformations are done in-place to avoid duplicating subtrees.

The reference imnplementation uses pointers and manual reference counting. Pointers in Rust are unsafe, so this implementation keeps all expressions in a vector which serves as an expression pool, and uses integer indices instead of pointers.

Instead of reference counting, this implementation uses garbage collection. Every now and then it finds all unreachable expressions and replaces them with special "Free" value.

## References
* [Lazy K specification](http://tromp.github.io/cl/lazy-k.html).
* [The original 2002 Esoteric Awards submission](http://esoteric.sange.fi/essie2/download/lazy-k/). This contains reference implementation in C++.
* [Lazy K - Esolang](https://esolangs.org/wiki/Lazy_K).
* [Source distribution by msullivan (C++)](https://github.com/msullivan/LazyK).
* [SKI combinator calculus](https://en.wikipedia.org/wiki/SKI_combinator_calculus).
