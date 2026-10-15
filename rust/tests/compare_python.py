"""Compare the Rust CLI against lisp.py on deterministic shared-language programs.

Run from the repository root after `cargo build --release --manifest-path rust/Cargo.toml`:
    python3 rust/tests/compare_python.py
"""
import importlib.util
from pathlib import Path
import random
import subprocess
import sys

ROOT = Path(__file__).resolve().parents[2]
SPEC = importlib.util.spec_from_file_location("reference_lisp", ROOT / "lisp.py")
REFERENCE = importlib.util.module_from_spec(SPEC)
sys.modules[SPEC.name] = REFERENCE
SPEC.loader.exec_module(REFERENCE)
BINARY = ROOT / "rust/target/release/lisp-rs"


def compare(source):
    output = []
    vm = REFERENCE.Interpreter(output=output.append, max_steps=1_000_000)
    value = vm.run(source)
    expected = output + ([] if value is None else [REFERENCE.display(value)])
    actual = subprocess.run([str(BINARY), "-e", source], capture_output=True, text=True, check=True)
    assert actual.stdout == "".join(line + "\n" for line in expected), (source, expected, actual.stdout)


def expression(rng, depth=0):
    if depth >= 3 or rng.random() < 0.3:
        return str(rng.randint(-9, 9))
    op = rng.choice(["+", "-", "*", "if"])
    left, right = expression(rng, depth + 1), expression(rng, depth + 1)
    if op == "if":
        return f"(if (< {rng.randint(-3, 3)} 0) {left} {right})"
    return f"({op} {left} {right})"


def main():
    programs = [
        "(+ (+ 1 2) (* 3 4) (+ 5 (* 6 7)))",
        "(list (+) (*) (- 4) (/ 2) (// -7 3) (% 7 -3))",
        "(list (and) (or) (and 1 2 3) (or false nil 4))",
        "(if true 42 (/ 1 0))",
        "'(1 (+ 2 3))",
        '(output "# not a comment; still text")',
        '(list (first "🙂ab") (rest "🙂ab") (length "你好🙂"))',
        "(set x 1) (function change () (set x 2)) (change) x",
        "(function make (x) (lambda (y) (+ x y))) (set add (make 10)) (add 7)",
        "(set i 0) (while (< i 20) (set i (+ i 1)))",
        (ROOT / "examples/fibonacci.lisp").read_text(),
        (ROOT / "examples/closures.lisp").read_text(),
    ]
    rng = random.Random(15112)
    programs.extend(expression(rng) for _ in range(100))
    for source in programs:
        compare(source)
    print(f"Matched Python output and result on {len(programs)} programs.")


if __name__ == "__main__":
    main()
