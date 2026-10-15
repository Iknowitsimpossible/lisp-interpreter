"""Behavioral tests for the language, diagnostics, tracing, and command line."""
import json
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest

from lisp import IncompleteInput, Interpreter, LispError, parse, tokenize, tree_data


ROOT = Path(__file__).resolve().parent


class LanguageTests(unittest.TestCase):
    def setUp(self):
        self.lines = []
        self.vm = Interpreter(output=self.lines.append)

    def test_supplied_nested_expression(self):
        self.assertEqual(self.vm.run('(+ (+ 1 2) (* 3 4) (+ 5 (* 6 7)))'), 62)

    def test_supplied_fibonacci_program(self):
        self.vm.run((ROOT / 'examples/fibonacci.lisp').read_text())
        self.assertEqual(self.lines, ['0 1', '1 1', '2 2', '3 3', '4 5',
                                     '5 8', '6 13', '7 21', '8 34', '9 55'])

    def test_tokenizer_preserves_comment_markers_in_strings(self):
        self.vm.run('(output "# is text; still text" "a\\nline") # ignored\n; ignored')
        self.assertEqual(self.lines, ['# is text; still text a\nline'])

    def test_escaped_quote(self):
        self.assertEqual(self.vm.run(r'"a\"b"'), 'a"b')

    def test_parse_tree(self):
        self.assertEqual(tree_data(parse(tokenize('(+ 1 (* 2 3))'))[0]),
                         ['+', 1, ['*', 2, 3]])

    def test_signed_and_decimal_numbers(self):
        self.assertEqual(self.vm.run('(+ -3 +4 .5)'), 1.5)

    def test_arithmetic_and_unary_operations(self):
        cases = {'(+)': 0, '(*)': 1, '(- 4)': -4, '(- 10 2 3)': 5,
                 '(/ 2)': .5, '(/ 20 2 5)': 2, '(// 9 2)': 4, '(% 9 2)': 1}
        for code, expected in cases.items():
            with self.subTest(code=code):
                self.assertEqual(self.vm.run(code), expected)

    def test_comparison_chain(self):
        self.assertIs(self.vm.run('(< 1 2 3)'), True)
        self.assertIs(self.vm.run('(<= 1 1 2)'), True)
        self.assertIs(self.vm.run('(= 3 3 4)'), False)

    def test_branch_is_lazy(self):
        self.assertEqual(self.vm.run('(if true 42 (/ 1 0))'), 42)
        self.assertIsNone(self.vm.run('(if false missing)'))

    def test_boolean_short_circuit(self):
        self.assertIs(self.vm.run('(and false missing)'), False)
        self.assertEqual(self.vm.run('(or 42 missing)'), 42)
        self.assertIs(self.vm.run('(and)'), True)
        self.assertIs(self.vm.run('(or)'), False)

    def test_quote_and_lists(self):
        self.assertEqual(self.vm.run("'(1 (+ 2 3))"), [1, ['+', 2, 3]])
        self.assertEqual(self.vm.run('(cons 1 (rest (list 9 2 3)))'), [1, 2, 3])
        self.assertEqual(self.vm.run('(length (list 1 2))'), 2)

    def test_multiple_top_level_expressions_and_persistence(self):
        self.assertEqual(self.vm.run('(set x 5) (+ x 2)'), 7)
        self.assertEqual(self.vm.run('x'), 5)

    def test_fresh_locals_on_recursive_calls(self):
        self.assertEqual(self.vm.run('''
            (function sum-down (n)
              (define saved n)
              (if (= n 0) 0 (+ (sum-down (- n 1)) saved)))
            (sum-down 10)
        '''), 55)

    def test_lexical_closure(self):
        self.assertEqual(self.vm.run('''
            (function make-adder (x) (lambda (y) (+ x y)))
            (set add-ten (make-adder 10))
            (function caller (x) (add-ten 7))
            (caller 100)
        '''), 17)

    def test_set_updates_nearest_binding_define_creates_local(self):
        self.assertEqual(self.vm.run('''
            (set x 1)
            (function change () (set x 2))
            (change)
            (function local () (define x 99) (set x 100))
            (local)
            x
        '''), 2)

    def test_mutable_closure(self):
        self.assertEqual(self.vm.run('''
            (function make-counter ()
              (define x 0)
              (lambda () (set x (+ x 1))))
            (set counter (make-counter))
            (counter)
            (counter)
        '''), 2)

    def test_step_limit_stops_empty_loop_body(self):
        vm = Interpreter(max_steps=30)
        with self.assertRaisesRegex(LispError, 'step limit'):
            vm.run('(while true (block))')

    def test_recursive_function_limit_and_cleanup(self):
        vm = Interpreter(max_call_depth=10)
        with self.assertRaisesRegex(LispError, 'call depth'):
            vm.run('(function again () (again)) (again)')
        self.assertEqual(vm.stack, [])
        self.assertEqual(vm.run('(+ 1 2)'), 3)

    def test_undefined_name_position(self):
        with self.assertRaisesRegex(LispError, "line 2, column 3: undefined name 'missing'"):
            self.vm.run('\n  missing')

    def test_runtime_errors(self):
        cases = ['(/ 1 0)', '(+ "text" 1)', '(+ true 1)', '(first (list))',
                 '(1 2)', '(if)', '(set 123 4)', '(lambda (x x) x)', '(first 1 2)']
        for code in cases:
            with self.subTest(code=code):
                with self.assertRaisesRegex(LispError, r'line 1, column \d+:'):
                    self.vm.run(code)

    def test_user_function_arity(self):
        with self.assertRaisesRegex(LispError, 'expects 1 arguments; got 0'):
            self.vm.run('(function f (x) x) (f)')

    def test_incomplete_and_malformed_syntax(self):
        for code in ['(', '(+ 1', "'", '"unfinished']:
            with self.subTest(code=code):
                with self.assertRaises(IncompleteInput):
                    self.vm.run(code)
        with self.assertRaisesRegex(LispError, "unexpected '\\)'"):
            self.vm.run(')')
        with self.assertRaisesRegex(LispError, 'invalid string escape'):
            self.vm.run(r'"bad\q"')

    def test_parser_nesting_limit(self):
        with self.assertRaisesRegex(LispError, 'nesting limit'):
            self.vm.run('(' * 160 + '1' + ')' * 160)

    def test_empty_program(self):
        self.assertIsNone(self.vm.run(' ; nothing here\n # or here'))

    def test_trace_records_scope_and_value_changes(self):
        vm = Interpreter(trace=True)
        self.assertEqual(vm.run('(function inc (x) (set x (+ x 1))) (inc 4)'), 5)
        call = next(event for event in vm.events if event['event'] == 'call')
        change = next(event for event in vm.events if event['event'] == 'set')
        returned = next(event for event in vm.events if event['event'] == 'return')
        self.assertEqual(call['bindings'], {'x': '4'})
        self.assertEqual((change['before'], change['after']), ('4', '5'))
        self.assertEqual(call['scope'], change['scope'])
        self.assertEqual(returned['value'], '5')
        json.dumps(vm.events)


class CommandLineTests(unittest.TestCase):
    def cli(self, *args, input=None):
        return subprocess.run([sys.executable, str(ROOT / 'lisp.py'), *args],
                              input=input, text=True, capture_output=True)

    def test_file_output_has_no_extra_value(self):
        result = self.cli(str(ROOT / 'examples/fibonacci.lisp'))
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(len(result.stdout.splitlines()), 10)
        self.assertEqual(result.stdout.splitlines()[-1], '9 55')

    def test_expression_and_ast(self):
        result = self.cli('-e', '(+ 1 2)')
        self.assertEqual((result.returncode, result.stdout), (0, '3\n'))
        ast = self.cli('-e', '(+ 1 2)', '--ast')
        self.assertEqual(json.loads(ast.stdout), [['+', 1, 2]])
        tokens = self.cli('-e', '(+ 1 2)', '--tokens')
        self.assertEqual(json.loads(tokens.stdout)[-1]['kind'], 'EOF')

    def test_repl_multiline(self):
        result = self.cli(input='(+ 1\n2)\n:quit\n')
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertIn('3\n', result.stdout)

    def test_cli_failure_and_partial_trace(self):
        with tempfile.TemporaryDirectory() as directory:
            trace = str(Path(directory) / 'trace.json')
            result = self.cli('-e', '(function bad () missing) (bad)', '--trace', trace)
            self.assertEqual(result.returncode, 1)
            self.assertIn("undefined name 'missing'", result.stderr)
            events = json.loads(Path(trace).read_text())
            self.assertTrue(any(event['event'] == 'error' for event in events))

    def test_missing_file_and_invalid_cli(self):
        self.assertEqual(self.cli('does-not-exist.lisp').returncode, 1)
        self.assertEqual(self.cli('-e', '1', '--max-steps', '0').returncode, 2)


if __name__ == '__main__':
    unittest.main()
