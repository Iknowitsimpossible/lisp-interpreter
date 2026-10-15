# A small Lisp interpreter you can inspect

A standalone interpreter for a small Lisp dialect, written in Python. It follows
**tokenize → parse → evaluate** and includes a recursive Fibonacci example.
There are no third-party dependencies. Use Python 3.9 or newer.

This is an educational Lisp dialect, not a complete implementation of Scheme or
Common Lisp. The GitHub projects
in RESEARCH.md were comparison references, not copied dependencies.

## Run it

Open a terminal in this folder, then:

```sh
python3 lisp.py examples/fibonacci.lisp
python3 lisp.py examples/closures.lisp
python3 lisp.py -e '(+ (+ 1 2) (* 3 4) (+ 5 (* 6 7)))'
python3 lisp.py
```

The first command prints:

```text
0 1
1 1
2 2
3 3
4 5
5 8
6 13
7 21
8 34
9 55
```

File execution prints only explicit `output` calls. `-e` and the REPL also show
the last non-nil expression value. Enter `:quit` to exit the REPL. An unfinished
parenthesized expression produces a continuation prompt.

## Look inside the interpreter

```sh
python3 lisp.py -e '(+ 1 (* 2 3))' --tokens
python3 lisp.py -e '(+ 1 (* 2 3))' --ast
python3 lisp.py examples/trace-demo.lisp --trace trace.json
python3 lisp.py -e '(while true (block))' --max-steps 100
python3 -m unittest -v
```

`--tokens` emits positioned tokens, including an EOF token. `--ast` emits nested
lists without executing the program. JSON represents both symbol names and string
contents as strings; internally they are distinct types.

`--trace` records evaluation locations, function entries and returns, parameter
bindings, and assignment before/after values. Each function invocation gets a
distinct scope number. A failure inside a user function records an error event,
and the CLI saves a partial trace even when evaluation fails. Values in trace
records are formatted text, not a complete serialization of interpreter state.
This is an execution log, not an interactive or reversible debugger.

## Supported language

| Category | Forms |
|---|---|
| Values | Integers, decimals, strings, `true`, `false`, `nil`, lists |
| Arithmetic | `+`, `-`, `*`, `/`, `//`, `%`; unary minus and reciprocal |
| Comparisons | `<`, `<=`, `>`, `>=`, `=`, `!=` |
| Control | `if`, `while`, `block` (also `begin` and `progn`) |
| Variables | `set`, `define` |
| Functions | `function`, `lambda`; recursion and lexical closures |
| Logic | `and`, `or`, `not`; `and` and `or` short-circuit |
| Lists | `quote` / `'`, `list`, `first`, `rest`, `cons`, `length` |
| Output | `(output value ...)` |
| Comments | `;` or `#` through end of line, except inside strings |

Expressions use **prefix notation**, not reverse Polish notation: `(+ 1 2)`.

```lisp
(block
  (function square (x) (* x x))
  (set answer (square 7))
  (output "answer:" answer))
```

### Exact choices made for this dialect

- A function returns the value of its final body expression. `if` evaluates only
  the chosen branch. An omitted false branch returns `nil`.
- A block runs expressions in order in the existing environment, returning the
  last value. It does not introduce a new scope.
- Each function call has a fresh local environment, linked to the environment
  where the function was defined. Parameters are local bindings.
- `define` creates or replaces a binding in the current environment. `set`
  updates the nearest existing binding, or creates one in the current environment
  if no binding exists. A function can therefore update an existing global.
- `function` binds a named function in the current environment; `lambda` returns
  an anonymous function. Functions are first-class values.
- Conditions use Python-style truth: `false`, `nil`, zero, empty strings, and
  empty lists are false. This intentionally differs from Scheme/Common Lisp.
- `=` compares adjacent values for equality; `!=` takes two arguments. Ordered
  comparisons accept two or more numeric arguments and check adjacent pairs.
- Strings use double quotes and JSON-style escapes such as `\n` and `\"`.
  Literal physical newlines inside strings are rejected. Use `true`/`false`, not
  Scheme's `#t`/`#f`, since `#` starts a comment here.
- `()` evaluates to an empty list. Lists are Python-backed, not cons-cell chains;
  `first`, `rest`, and `length` also accept strings where the operation makes sense.

## Organization

- `lisp.py`: positioned tokenizer, recursive parser, environments, evaluator, CLI,
  and multiline REPL.
- `test_lisp.py`: 30 tests, including your examples, lexical closures, recursive
  local variables, short-circuiting, errors, limits, trace data, and CLI behavior.
- `examples/`: Fibonacci, closures/factorial, and a small trace example.
- `RESEARCH.md`: GitHub comparison and carefully qualified extension proposals.

## Limits and next steps

The evaluator is a recursive tree walker, with a default 100,000-expression step
budget and an 80-function-call depth limit. The parser limits nesting to 150.
These make common accidental infinite loops and deep recursion easier to diagnose;
they are not a CPU/memory sandbox. Individual arithmetic or string operations can
still be expensive. The guest language has no file, network, import, or Python-eval
primitive.

Not implemented: tail-call optimization, macros, quasiquotation, a bytecode
compiler, Scheme/Common Lisp compatibility, a visual debugger, or live pause/
resume. These are future extensions, not claims of novel inventions.

Validation: all 30 tests passed on the creation host. Your Fibonacci example
printed exactly ten lines, ending with `9 55`.
