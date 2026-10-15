"""Small CLI comparison, including startup and parsing; not a general benchmark.

Build Rust in release mode first, then run: python3 rust/benchmark.py
"""
import json
from pathlib import Path
import platform
import statistics
import subprocess
import sys
import time

ROOT = Path(__file__).resolve().parents[1]
PROGRAMS = {
    "recursive_fib20": "(function fib (n) (if (< n 2) 1 (+ (fib (- n 1)) (fib (- n 2))))) (fib 20)",
    "increment_loop_10000": "(set i 0) (while (< i 10000) (set i (+ i 1)))",
}


def main():
    commands = {"python": [sys.executable, str(ROOT / "lisp.py")],
                "rust": [str(ROOT / "rust/target/release/lisp-rs")]}
    report = {"machine": platform.machine(), "python": platform.python_version(),
              "method": "median of 5 CLI runs after one warmup; includes startup, parsing and execution; tracing disabled",
              "results": {}}
    for name, source in PROGRAMS.items():
        timings, outputs = {}, {}
        for language, command in commands.items():
            args = command + ["-e", source, "--max-steps", "5000000"]
            subprocess.run(args, capture_output=True, text=True, check=True)
            samples = []
            for _ in range(5):
                start = time.perf_counter()
                result = subprocess.run(args, capture_output=True, text=True, check=True)
                samples.append(time.perf_counter() - start)
            timings[language] = statistics.median(samples)
            outputs[language] = result.stdout
        assert outputs["python"] == outputs["rust"]
        report["results"][name] = {"median_seconds": timings,
                                   "python_time_divided_by_rust_time": timings["python"] / timings["rust"],
                                   "result": outputs["rust"].strip()}
    print(json.dumps(report, indent=2))


if __name__ == "__main__":
    main()
