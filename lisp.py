#!/usr/bin/env python3
"""A small teaching Lisp: tokenize -> parse -> evaluate. Python 3.9+, no dependencies."""

import argparse
from dataclasses import dataclass
import functools
import json
import operator
from pathlib import Path
import re
import sys


class LispError(Exception):
    pass


class IncompleteInput(LispError):
    """The REPL should request another line."""


class Symbol(str):
    """Distinguish variable names from string literals."""


@dataclass
class Token:
    kind: str
    value: object
    line: int
    column: int


@dataclass
class Node:
    value: object
    line: int
    column: int

    def fail(self, message):
        raise LispError(f"line {self.line}, column {self.column}: {message}")


def tokenize(code):
    """Read comments, quoted strings, punctuation, numbers, and symbols."""
    tokens = []
    i, line, column = 0, 1, 1
    while i < len(code):
        char = code[i]
        if char.isspace():
            if char == '\n':
                line, column = line + 1, 1
            else:
                column += 1
            i += 1
        elif char in ';#':
            # This dialect uses # and ; comments, and true/false (not #t/#f).
            while i < len(code) and code[i] != '\n':
                i, column = i + 1, column + 1
        elif char in "()'":
            tokens.append(Token(char, char, line, column))
            i, column = i + 1, column + 1
        elif char == '"':
            start, start_column = i, column
            i += 1
            while i < len(code):
                if code[i] == '\n':
                    raise LispError(f"line {line}, column {start_column}: use \\n inside strings")
                if code[i] == '\\':
                    i += 2
                elif code[i] == '"':
                    break
                else:
                    i += 1
            if i >= len(code):
                raise IncompleteInput(f"line {line}, column {start_column}: unclosed string")
            raw = code[start:i + 1]
            try:
                value = json.loads(raw)
            except ValueError as error:
                raise LispError(f"line {line}, column {start_column}: invalid string escape") from error
            tokens.append(Token('atom', value, line, start_column))
            column += len(raw)
            i += 1
        else:
            start, start_column = i, column
            while i < len(code) and not code[i].isspace() and code[i] not in "()'\";#":
                if ord(code[i]) < 32:
                    raise LispError(f"line {line}, column {column}: illegal control character")
                i, column = i + 1, column + 1
            text = code[start:i]
            if re.fullmatch(r'[+-]?\d+', text):
                try:
                    value = int(text)
                except ValueError as error:
                    raise LispError(f"line {line}, column {start_column}: integer literal too long") from error
            elif re.fullmatch(r'[+-]?(?:\d+\.\d*|\.\d+)', text):
                value = float(text)
            else:
                value = Symbol(text)
            tokens.append(Token('atom', value, line, start_column))
    tokens.append(Token('EOF', None, line, column))
    return tokens


def parse(tokens):
    """Build positioned expression trees without popping the front of a list."""
    index = 0

    def read(depth=0):
        nonlocal index
        token = tokens[index]
        if depth > 150:
            raise LispError(f"line {token.line}, column {token.column}: nesting limit exceeded")
        index += 1
        if token.kind == '(':
            children = []
            while tokens[index].kind != ')':
                if tokens[index].kind == 'EOF':
                    raise IncompleteInput(f"line {token.line}, column {token.column}: unclosed '('")
                children.append(read(depth + 1))
            index += 1
            return Node(children, token.line, token.column)
        if token.kind == "'":
            return Node([Node(Symbol('quote'), token.line, token.column), read(depth + 1)],
                        token.line, token.column)
        if token.kind == ')':
            raise LispError(f"line {token.line}, column {token.column}: unexpected ')'")
        if token.kind == 'EOF':
            raise IncompleteInput(f"line {token.line}, column {token.column}: expected expression")
        return Node(token.value, token.line, token.column)

    program = []
    while tokens[index].kind != 'EOF':
        program.append(read())
    return program


def tree_data(node):
    """Expose a simple nested list representation for --ast."""
    if isinstance(node.value, list):
        return [tree_data(child) for child in node.value]
    return node.value


def display(value):
    if value is None:
        return 'nil'
    if value is True:
        return 'true'
    if value is False:
        return 'false'
    if isinstance(value, list):
        return '(' + ' '.join(display(item) for item in value) + ')'
    if isinstance(value, Function):
        return f'<function {value.name}>'
    if isinstance(value, Builtin):
        return f'<builtin {value.name}>'
    return str(value)


class Environment:
    def __init__(self, parent=None, bindings=None, scope=0):
        self.parent = parent
        self.bindings = {} if bindings is None else bindings
        self.scope = scope

    def owner(self, name):
        env = self
        while env is not None:
            if name in env.bindings:
                return env
            env = env.parent
        return None

    def get(self, name, node):
        owner = self.owner(name)
        if owner is None:
            node.fail(f"undefined name '{name}'")
        return owner.bindings[name]


@dataclass
class Function:
    name: str
    parameters: list
    body: list
    environment: Environment


@dataclass
class Builtin:
    name: str
    operation: object
    minimum: int
    maximum: object = None

    def apply(self, arguments, node):
        if len(arguments) < self.minimum or (self.maximum is not None and len(arguments) > self.maximum):
            maximum = 'unlimited' if self.maximum is None else self.maximum
            node.fail(f"{self.name} expects {self.minimum}..{maximum} arguments; got {len(arguments)}")
        return self.operation(*arguments)


def numbers(operation):
    def checked(*values):
        if any(type(value) not in (int, float) for value in values):
            raise TypeError('expected numeric arguments')
        return operation(*values)
    return checked


class Interpreter:
    def __init__(self, output=print, max_steps=100000, trace=False, max_call_depth=80):
        if max_steps < 1 or not 1 <= max_call_depth <= 80:
            raise ValueError('max_steps must be positive; max_call_depth must be 1..80')
        self.output = output
        self.max_steps = max_steps
        self.max_call_depth = max_call_depth
        self.tracing = trace
        self.events = []
        self.steps = 0
        self.stack = []
        self.next_scope = 1
        self.globals = Environment()
        self._install_builtins()

    def _install_builtins(self):
        def builtin(name, operation, minimum, maximum=None):
            self.globals.bindings[name] = Builtin(name, operation, minimum, maximum)

        builtin('+', numbers(lambda *xs: sum(xs)), 0)
        builtin('*', numbers(lambda *xs: functools.reduce(operator.mul, xs, 1)), 0)
        builtin('-', numbers(lambda x, *xs: -x if not xs else x - sum(xs)), 1)
        builtin('/', numbers(lambda x, *xs: 1 / x if not xs else functools.reduce(operator.truediv, xs, x)), 1)
        builtin('//', numbers(operator.floordiv), 2, 2)
        builtin('%', numbers(operator.mod), 2, 2)
        for name, compare in [('<', operator.lt), ('<=', operator.le), ('>', operator.gt), ('>=', operator.ge)]:
            builtin(name, numbers(lambda *xs, compare=compare:
                                  all(compare(a, b) for a, b in zip(xs, xs[1:]))), 2)
        builtin('=', lambda *xs: all(x == xs[0] for x in xs[1:]), 2)
        builtin('!=', operator.ne, 2, 2)
        builtin('not', operator.not_, 1, 1)
        builtin('list', lambda *xs: list(xs), 0)
        builtin('first', lambda xs: xs[0], 1, 1)
        builtin('rest', lambda xs: xs[1:], 1, 1)
        builtin('length', len, 1, 1)
        builtin('cons', lambda x, xs: [x] + xs, 2, 2)
        builtin('output', lambda *xs: self.output(' '.join(display(x) for x in xs)), 0)
        self.globals.bindings.update({'true': True, 'false': False, 'nil': None})

    def event(self, kind, node, env, **extra):
        if self.tracing:
            self.events.append({'event': kind, 'step': self.steps, 'line': node.line,
                                'column': node.column, 'scope': env.scope,
                                'stack': self.stack.copy(), **extra})

    def run(self, code):
        self.steps = 0
        self.events = []
        try:
            value = None
            for node in parse(tokenize(code)):
                value = self.evaluate(node, self.globals)
            return value
        except RecursionError as error:
            raise LispError('expression recursion limit exceeded; simplify the expression') from error

    @staticmethod
    def arity(node, arguments, minimum, maximum=None):
        if len(arguments) < minimum or (maximum is not None and len(arguments) > maximum):
            node.fail('wrong number of arguments for special form')

    @staticmethod
    def name(node):
        if not isinstance(node.value, Symbol):
            node.fail('expected a variable or function name')
        return str(node.value)

    def evaluate(self, node, env):
        self.steps += 1
        if self.steps > self.max_steps:
            node.fail(f'execution step limit exceeded ({self.max_steps})')
        self.event('evaluate', node, env)
        try:
            return self._evaluate(node, env)
        except LispError:
            raise
        except (TypeError, ValueError, ArithmeticError, IndexError, KeyError) as error:
            node.fail(str(error))

    def _evaluate(self, node, env):
        value = node.value
        if isinstance(value, Symbol):
            return env.get(str(value), node)
        if not isinstance(value, list):
            return value
        if not value:
            return []
        head, args = value[0], value[1:]
        command = str(head.value) if isinstance(head.value, Symbol) else None

        if command == 'quote':
            self.arity(node, args, 1, 1)
            return tree_data(args[0])
        if command in ('block', 'begin', 'progn'):
            result = None
            for expression in args:
                result = self.evaluate(expression, env)
            return result
        if command == 'if':
            self.arity(node, args, 2, 3)
            if self.evaluate(args[0], env):
                return self.evaluate(args[1], env)
            return self.evaluate(args[2], env) if len(args) == 3 else None
        if command in ('and', 'or'):
            result = command == 'and'
            for expression in args:
                result = self.evaluate(expression, env)
                if (command == 'and' and not result) or (command == 'or' and result):
                    break
            return result
        if command == 'while':
            self.arity(node, args, 2)
            result = None
            while self.evaluate(args[0], env):
                for expression in args[1:]:
                    result = self.evaluate(expression, env)
            return result
        if command in ('set', 'define'):
            self.arity(node, args, 2, 2)
            name = self.name(args[0])
            result = self.evaluate(args[1], env)
            owner = env if command == 'define' else (env.owner(name) or env)
            before = display(owner.bindings[name]) if name in owner.bindings else '<unbound>'
            owner.bindings[name] = result
            self.event('set', node, owner, name=name, before=before, after=display(result))
            return result
        if command in ('function', 'lambda'):
            self.arity(node, args, 3 if command == 'function' else 2)
            if command == 'function':
                name = self.name(args[0])
                params_node, body = args[1], args[2:]
            else:
                name, params_node, body = '<lambda>', args[0], args[1:]
            if not isinstance(params_node.value, list):
                params_node.fail('expected a parenthesized parameter list')
            params = [self.name(parameter) for parameter in params_node.value]
            if len(set(params)) != len(params):
                params_node.fail('duplicate parameter name')
            function = Function(name, params, body, env)
            if command == 'function':
                env.bindings[name] = function
            return function

        function = self.evaluate(head, env)
        if not isinstance(function, (Builtin, Function)):
            head.fail('attempted to call a non-function')
        arguments = [self.evaluate(argument, env) for argument in args]
        if isinstance(function, Builtin):
            return function.apply(arguments, node)
        if len(arguments) != len(function.parameters):
            node.fail(f'{function.name} expects {len(function.parameters)} arguments; got {len(arguments)}')
        if len(self.stack) >= self.max_call_depth:
            node.fail(f'function call depth limit exceeded ({self.max_call_depth})')
        local = Environment(function.environment, dict(zip(function.parameters, arguments)), self.next_scope)
        self.next_scope += 1
        self.stack.append(function.name)
        self.event('call', node, local, function=function.name,
                   bindings={name: display(item) for name, item in local.bindings.items()})
        try:
            result = None
            for expression in function.body:
                result = self.evaluate(expression, local)
            self.event('return', node, local, function=function.name, value=display(result))
            return result
        except LispError as error:
            self.event('error', node, local, message=str(error))
            raise
        finally:
            self.stack.pop()


def repl(interpreter):
    print('Teaching Lisp. Enter :quit to exit. Multiline expressions are supported.')
    pending = ''
    while True:
        try:
            line = input('... ' if pending else 'lisp> ')
            if not pending and line.strip() == ':quit':
                return
            pending += line + '\n'
            try:
                parse(tokenize(pending))
            except IncompleteInput:
                continue
            result = interpreter.run(pending)
            if result is not None:
                print(display(result))
            pending = ''
        except EOFError:
            if pending:
                print('Incomplete input discarded.', file=sys.stderr)
            return
        except KeyboardInterrupt:
            pending = ''
            print()
        except LispError as error:
            print(f'Error: {error}', file=sys.stderr)
            pending = ''


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('file', nargs='?', help='Lisp file; omit to start the REPL')
    parser.add_argument('-e', '--expression', help='run source text instead of a file')
    parser.add_argument('--tokens', action='store_true', help='print tokens without running')
    parser.add_argument('--ast', action='store_true', help='print parse trees without running')
    parser.add_argument('--trace', type=Path, help='write execution events to a JSON file')
    parser.add_argument('--max-steps', type=int, default=100000)
    options = parser.parse_args()
    if options.file and options.expression is not None:
        parser.error('choose either a file or -e')
    if options.max_steps < 1:
        parser.error('--max-steps must be positive')
    if options.file is None and options.expression is None and (options.tokens or options.ast or options.trace):
        parser.error('--tokens, --ast, and --trace require a file or -e')
    interpreter = Interpreter(max_steps=options.max_steps, trace=options.trace is not None)
    try:
        if options.file is None and options.expression is None:
            repl(interpreter)
            return 0
        source = Path(options.file).read_text() if options.file else options.expression
        if options.tokens:
            print(json.dumps([vars(token) for token in tokenize(source)], indent=2))
        elif options.ast:
            print(json.dumps([tree_data(node) for node in parse(tokenize(source))], indent=2))
        else:
            result = interpreter.run(source)
            if result is not None and options.expression is not None:
                print(display(result))
        return 0
    except (LispError, OSError) as error:
        print(f'Error: {error}', file=sys.stderr)
        return 1
    finally:
        if options.trace:
            try:
                options.trace.write_text(json.dumps(interpreter.events, indent=2) + '\n')
            except OSError as error:
                print(f'Could not write trace: {error}', file=sys.stderr)
                # A requested trace is part of the result; report its failure to callers.
                raise SystemExit(1)


if __name__ == '__main__':
    sys.exit(main())
