# Language reference


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

## Execution limits

The evaluator is a recursive tree walker, with a default 100,000-expression step
budget and an 80-function-call depth limit. The parser limits nesting to 150.
These make common accidental infinite loops and deep recursion easier to diagnose;
they are not a CPU/memory sandbox. Individual arithmetic or string operations can
still be expensive. The guest language has no file, network, import, or Python-eval
primitive.

Not implemented: tail-call optimization, macros, quasiquotation, a bytecode
compiler, Scheme/Common Lisp compatibility, a visual debugger, or live pause/
resume. These are future extensions, not claims of novel inventions.
