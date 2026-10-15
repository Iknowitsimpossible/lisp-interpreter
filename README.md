# Lisp Interpreter

A small Lisp with two implementations: a **Rust bytecode VM** and a **Python
reference interpreter**. Both support recursion, lexical closures, and execution traces.

## Quick start

```sh
git clone --branch rust-version https://github.com/Iknowitsimpossible/lisp-interpreter.git
cd lisp-interpreter
```

With [Rust](https://rustup.rs/) installed, run the Fibonacci example or open the REPL:

```sh
cargo run --release --manifest-path rust/Cargo.toml -- examples/fibonacci.lisp
cargo run --release --manifest-path rust/Cargo.toml
```

Or use Python 3.9+ without dependencies:

```sh
python3 lisp.py examples/fibonacci.lisp
python3 lisp.py
```

Enter `:quit` to leave either REPL.

## Example

```lisp
(function square (x) (* x x))
(output (square 7))  ; 49
```

## Features

- Variables, conditionals, loops, lists, and first-class functions.
- Recursive functions and lexical closures.
- Source-positioned errors, token/parse-tree inspection, and JSON execution traces.
- Rust: bytecode compilation, an explicit call stack, and environment garbage collection.

## Tests

```sh
cargo test --manifest-path rust/Cargo.toml
python3 -m unittest -v
cargo build --release --manifest-path rust/Cargo.toml
python3 rust/tests/compare_python.py
```

See the [Rust guide](rust/README.md) for tracing, embedding, and compatibility differences,
the [language reference](LANGUAGE.md) for the shared syntax,
and [examples](examples/) for programs. These are educational dialects, not full
Scheme or Common Lisp. [Research notes](RESEARCH.md) cover related projects.
