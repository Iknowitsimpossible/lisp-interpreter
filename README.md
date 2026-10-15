# Lisp Interpreter

A small Lisp interpreter written in Python, with recursion, lexical closures,
and execution tracing. No dependencies; requires Python 3.9+.

## Quick start

```sh
git clone https://github.com/Iknowitsimpossible/lisp-interpreter.git
cd lisp-interpreter
python3 lisp.py
```

Type expressions at the prompt, or enter `:quit` to exit:

```lisp
(+ 2 (* 3 4))  ; 14

(function square (x) (* x x))
(square 7)     ; 49
```

Run an example or a single expression:

```sh
python3 lisp.py examples/fibonacci.lisp
python3 lisp.py -e '(+ 1 2)'
```

## Features

- Variables, conditionals, loops, and first-class functions.
- Recursion and lexical closures.
- Numbers, strings, lists, and comments.
- Errors with line and column numbers.
- Token and parse-tree inspection; JSON traces of calls and variable changes.

The implementation follows **tokenize → parse → evaluate**.

## Inspect execution

```sh
python3 lisp.py -e '(+ 1 (* 2 3))' --tokens
python3 lisp.py -e '(+ 1 (* 2 3))' --ast
python3 lisp.py examples/trace-demo.lisp --trace trace.json
```

## Tests

```sh
python3 -m unittest -v
```

A small educational dialect, rather than full Scheme or Common Lisp.
See the [language reference](LANGUAGE.md) for syntax and execution limits,
[examples](examples/) for programs, and [research notes](RESEARCH.md) for related projects.
