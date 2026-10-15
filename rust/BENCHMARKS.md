# Small local performance comparison

Measured on October 15, 2026, on an arm64 macOS host, using Python 3.13.15 and
Rust 1.99.0. Rust used the release build with the Cargo settings in this repository.
Tracing was disabled in both implementations.

Each number is the median wall time of five separate CLI runs after one warmup.
This includes process startup, parsing, compilation where applicable, and execution.
The result text was checked for equality. Both CLIs were passed a 5,000,000-step
budget, although their definitions of a step differ.

| Program | Python | Rust | Python time / Rust time |
|---|---:|---:|---:|
| Recursive Fibonacci, `fib(20)` → `10946` | 166.0 ms | 33.6 ms | 4.94× |
| Incrementing loop, 10,000 iterations → `10000` | 85.9 ms | 9.2 ms | 9.38× |

These are two tiny programs, not a representative benchmark suite. Results depend
on hardware, toolchain, system load, and workload. They do not establish a general
speedup for Rust or a ranking against other Lisp implementations. They compare
this Rust bytecode VM with this Python tree walker, so both host language and
execution architecture changed. No claim about JIT or native Lisp compilation is
implied.

To repeat the same method from the repository root:

```sh
cargo build --release --manifest-path rust/Cargo.toml
python3 rust/benchmark.py
```
