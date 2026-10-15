# GitHub research: what already exists, and where to take this project

Research performed October 15, 2026. This is a focused survey of public projects,
not an exhaustive search or a proof of originality. Feature descriptions below
come from the linked projects' source pages or documentation. I did not install
or benchmark these external projects. Search results and repository contents can
change.

## Existing work

| Project | What the inspected material establishes | What it means for this project |
|---|---|---|
| [Peter Norvig's Lispy](https://github.com/norvig/pytudes/blob/main/py/lis.py), with [author's explanation](https://www.norvig.com/lispy.html) | A compact Python interpreter for a Scheme subset, with environments and user-defined procedures. | Another small Python tree-walking Lisp is a learning exercise, not a new category. |
| [Make a Lisp (MAL)](https://github.com/kanaka/mal), with [implementation guide](https://github.com/kanaka/mal/blob/master/process/guide.md) | An incremental learning project with steps covering environments, functions, tail calls, quoting, macros, exceptions, and self-hosting. | Adding macros, tail-call optimization, or writing it in another host language is established work. |
| [TinyLisp](https://github.com/Robert-van-Engelen/tinylisp) | A 99-line C implementation, including primitives, garbage collection, and a REPL; the repository also describes tail-call-optimized versions. | Small size and tail-call optimization alone are not a distinctive direction. |
| [CodeTracer](https://github.com/metacraft-labs/codetracer) | A time-travel debugger using recorded execution; its documentation includes a miniature Lisp-like language called `small`. | Recording execution or adding backward inspection is not an untouched idea, including in Lisp-like teaching examples. |
| [Longtable](https://github.com/ndouglas/longtable) | A Lisp-like simulation/rule system documenting persistent snapshots, rollback, branching, and explanations of state. | State history, rollback, and “why” explanations already have related implementations. |

## What has not been done yet?

The honest answer is that this survey cannot establish that a broad Lisp feature
has never been done. Basic interpretation, recursion, closures, macros, compilation,
and tracing already have substantial prior work. A repository not mentioning a
feature is not proof that its code lacks it.

The more useful question is: **what specific learning workflow could this project
serve better?** My recommendation is a teaching interpreter tailored to your
course dialect, organized around the question “why did this expression produce
this value?” The following are proposed directions, not verified original ideas:

1. **Connect source, tree, and variables in one view.** Selecting `(fib 3)` could
   highlight its source, corresponding parse subtree, active parameter `n`, and
   return value. The current implementation supplies positioned trees and trace
   events as foundations. It does not yet supply the visual interface.
2. **Ask for predictions before revealing execution.** Pause at an `if`, function
   call, or variable update; ask the learner for the next value; then reveal the
   actual result and a short explanation. Include exercises about fresh local
   variables versus globals. This prediction workflow is not implemented here.
3. **Compare interpreter and compiler on the same tiny program.** Keep this tree
   evaluator as a reference, add a small bytecode compiler/VM, and compare outputs
   and variable updates across automatically generated test programs. Show both
   paths to explain execution versus translation. The compiler and comparison
   harness remain unbuilt here; the technique itself is established.

The inspected material does not establish a complete implementation of that exact
course-specific combination. This is a product opportunity hypothesis, not a
claim of a research gap. Before describing it publicly as “the first,” search
education tools, Scheme environment-model visualizers, debugger research, and
additional repository source code in detail.

## What is actually delivered

A dependency-free Python interpreter matching your supplied Fibonacci syntax,
with lexical environments, closures, positioned errors, tokens/AST inspection,
and JSON execution events. These are useful foundations, not a claim that the
language or its features are new. The original implementation here does not
depend on the surveyed projects.

The concrete missing features in **this deliverable** are listed in README.md:
tail-call optimization, macros, bytecode compilation, compatibility with standard
Lisp dialects, and an interactive visual debugger. They already exist in other
projects, so implementing them would extend this project rather than invent them.
