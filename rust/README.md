# Rust interpreter

The Rust implementation compiles Lisp expressions to bytecode and executes them
on a virtual machine. The Python interpreter remains available in the repository
root as a readable reference.

## Run

Install a current stable [Rust toolchain](https://rustup.rs/), then run these commands
from the repository root:

```sh
cargo run --release --manifest-path rust/Cargo.toml -- examples/fibonacci.lisp
cargo run --release --manifest-path rust/Cargo.toml -- -e '(+ 2 (* 3 4))'
cargo run --release --manifest-path rust/Cargo.toml
```

The final command starts a multiline REPL. Enter `:quit` to exit.
The Fibonacci example prints ten pairs, from `0 1` through `9 55`.

## Inspect execution

```sh
cargo run --release --manifest-path rust/Cargo.toml -- -e '(+ 1 2)' --tokens
cargo run --release --manifest-path rust/Cargo.toml -- -e '(+ 1 2)' --ast
cargo run --release --manifest-path rust/Cargo.toml -- examples/trace-demo.lisp --trace trace.json
```

`--trace` writes instruction steps, calls, returns, parameter bindings, variable
changes, and runtime errors. Scope numbers remain unique even when environment
storage is reused. Traces contain rendered values, not a resumable machine image.
An error still saves partial output and trace events.

## Implementation

- `src/syntax.rs`: tokens, source positions, quoted strings, and expression trees.
- `src/vm.rs`: compiler, operand stack, explicit function-call stack, lexical
  environments, and tracing.
- `src/main.rs`: file execution, expression evaluation, and REPL.
- `src/lib.rs`: reusable Rust library API.

The compiler emits loads, stores, calls, branches, and returns. Loops and Lisp
function calls do not recurse through Rust's native call stack. Anonymous and
named functions capture their defining environment; each call gets fresh locals.
An environment arena with mark-and-sweep collection reclaims unused call frames
while retaining environments reachable through active calls, global values,
operand values, and closures (including closures held in lists).

The sole direct dependency is `serde_json`, used for JSON strings and inspection
output. `Cargo.lock` records the dependency resolution.

## Embedding

```rust
use lisp_interpreter::Interpreter;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut interpreter = Interpreter::new();
    let result = interpreter.run("(output 7) (+ 1 2)")?;
    assert_eq!(result.output, ["7"]);
    assert_eq!(result.value, "3");
    Ok(())
}
```

Globals persist between calls to `run`. Results and output are returned as text;
internal garbage-collected values do not escape through this API. Output is
buffered until a run completes or fails. Native game-command registration is a
future extension, not part of this API yet.

## Compatibility and limits

The [language reference](../LANGUAGE.md) describes the shared syntax and scope
rules. Important differences from the Python implementation:

- Integers are signed 64-bit values with checked arithmetic. Overflow reports an
  error; Python uses arbitrary-size integers. Decimals use finite 64-bit floats.
- Special forms are validated at compile time, including function bodies and
  branches that may never execute. Python validates them during evaluation.
- The default budget is 1,000,000 **bytecode instructions**, adjustable with
  `--max-steps`. Python counts expressions, so budget values are not equivalent.
- The default function-call depth limit is 1,000, adjustable with
  `--max-call-depth`. The parser limits expression nesting to 150.
- Trace `step` events refer to VM instructions, not Python's `evaluate` events.
- Interrupting the REPL with Ctrl-C exits the Rust process; Python clears the
  current input and continues.

The runtime has no macros, tail-call optimization, JIT, or full Scheme/Common
Lisp compatibility. Execution limits are not a CPU or memory sandbox. Compiling
the interpreter to a native binary does not compile Lisp programs to native code.

## Checks

```sh
cargo fmt --manifest-path rust/Cargo.toml --check
cargo clippy --manifest-path rust/Cargo.toml --all-targets -- -D warnings
cargo test --manifest-path rust/Cargo.toml
cargo build --release --manifest-path rust/Cargo.toml
python3 rust/tests/compare_python.py
```

The Rust suite has 33 integration tests. The comparison script checks output and
results against Python on 112 deterministic programs, including both supplied
example files and 100 generated arithmetic/conditional expressions. This is
coverage of the shared subset, not a proof of complete equivalence.

See [local performance measurements](BENCHMARKS.md) for two small, reproducible
comparisons against the Python tree walker and the limits of those measurements.
