# LazyK-rust
Interpreter for Lazy K programming language, written in Rust.

[Lazy K](http://tromp.github.io/cl/lazy-k.html) is an esoteric pure functional programming language designed by Ben Rudiak-Gould. 


## Usage

The following command runs Lazy program, reading from the standard input and writing to the standard output:
```
cargo run <path_to_source>
```

## TODO:

* Properly handle memory, without unsafe and with reference counting.
* Refactor: sparate Interpreter, Parser, Io.
* Find a way to have shared expression pool and constants without putting everything in a single class.
* Separate into library and a binary.
* Make sure behaviour of binary is as close as possible to the reference implementation.
* Add more tests, move them out of source.
* Do some profiling, compare to the reference implementation. This should perform at least as fast and use at least as much memory.


## References
* [Lazy K specification](http://tromp.github.io/cl/lazy-k.html)
* [The original 2002 Esoteric Awards submission](http://esoteric.sange.fi/essie2/download/lazy-k/). This contains reference implementation in C++.
* [Lazy K - Esolang](https://esolangs.org/wiki/Lazy_K)
* [Source distribution by msullivan (C++)](https://github.com/msullivan/LazyK)

All examples in this repository (except "Hello, world") are taken from the original submission. The "Hello, world" example can be found in the specification.