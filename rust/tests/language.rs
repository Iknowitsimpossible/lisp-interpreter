use lisp_interpreter::{inspect_ast, inspect_tokens, Interpreter};
use serde_json::json;

fn value(source: &str) -> String {
    Interpreter::new().run(source).unwrap().value
}

#[test]
fn supplied_nested_expression() {
    assert_eq!(value("(+ (+ 1 2) (* 3 4) (+ 5 (* 6 7)))"), "62");
}

#[test]
fn original_fibonacci_program() {
    let result = Interpreter::new()
        .run(include_str!("../../examples/fibonacci.lisp"))
        .unwrap();
    assert_eq!(
        result.output,
        vec!["0 1", "1 1", "2 2", "3 3", "4 5", "5 8", "6 13", "7 21", "8 34", "9 55"]
    );
}

#[test]
fn closures_and_factorial_example() {
    let result = Interpreter::new()
        .run(include_str!("../../examples/closures.lisp"))
        .unwrap();
    assert_eq!(result.output, vec!["10 + 7 = 17", "6! = 720"]);
}

#[test]
fn strings_comments_unicode_and_escapes() {
    let result = Interpreter::new()
        .run("(output \"# text; still text\" \"a\\nline\") # comment\n; ignored")
        .unwrap();
    assert_eq!(result.output, vec!["# text; still text a\nline"]);
    assert_eq!(value(r#""a\"b""#), "a\"b");
    assert_eq!(value("(length \"你好🙂\")"), "3");
    assert_eq!(value("(first \"🙂a\")"), "🙂");
    assert_eq!(value("(rest \"🙂a\")"), "a");
}

#[test]
fn json_inspection() {
    assert_eq!(
        inspect_ast("(+ 1 (* 2 3))").unwrap(),
        json!([["+", 1, ["*", 2, 3]]])
    );
    assert_eq!(
        inspect_tokens("(+ 1 2)")
            .unwrap()
            .as_array()
            .unwrap()
            .last()
            .unwrap()["kind"],
        "EOF"
    );
}

#[test]
fn arithmetic_identity_unary_and_floats() {
    for (code, expected) in [
        ("(+)", "0"),
        ("(*)", "1"),
        ("(- 4)", "-4"),
        ("(- 10 2 3)", "5"),
        ("(/ 2)", "0.5"),
        ("(/ 20 2 5)", "2.0"),
        ("(+ -3 +4 .5)", "1.5"),
        ("(// 9 2)", "4"),
        ("(% 9 2)", "1"),
    ] {
        assert_eq!(value(code), expected, "{code}");
    }
}

#[test]
fn negative_division_matches_python() {
    for (code, expected) in [
        ("(// -7 3)", "-3"),
        ("(% -7 3)", "2"),
        ("(// 7 -3)", "-3"),
        ("(% 7 -3)", "-2"),
        ("(% -7 -3)", "-1"),
        ("(// -7.0 3)", "-3.0"),
        ("(% -7.0 3)", "2.0"),
    ] {
        assert_eq!(value(code), expected, "{code}");
    }
}

#[test]
fn exact_integer_comparison_and_equality() {
    assert_eq!(value("(< 9007199254740992 9007199254740993)"), "true");
    assert_eq!(value("(= 9007199254740993 9007199254740992.0)"), "false");
    assert_eq!(
        value("(= 9223372036854775807 9223372036854775808.0)"),
        "false"
    );
    assert_eq!(value("(> 9007199254740993 9007199254740992.0)"), "true");
    assert_eq!(value("(< 9007199254740992.0 9007199254740993)"), "true");
    assert_eq!(
        value("(< 9223372036854775807 9223372036854775808.0)"),
        "true"
    );
    assert_eq!(value("(= true 1)"), "true");
    assert_eq!(value("(< 1 2 3)"), "true");
    assert_eq!(value("(<= 1 1 2)"), "true");
    assert_eq!(value("(= 3 3 4)"), "false");
}

#[test]
fn lazy_branch_and_boolean_operators() {
    for (code, expected) in [
        ("(if true 42 (/ 1 0))", "42"),
        ("(if false missing)", "nil"),
        ("(and false missing)", "false"),
        ("(or 42 missing)", "42"),
        ("(and)", "true"),
        ("(or)", "false"),
        ("(and 1 2 3)", "3"),
        ("(or false nil 0 4)", "4"),
        ("(not 0)", "true"),
    ] {
        assert_eq!(value(code), expected, "{code}");
    }
}

#[test]
fn quote_and_list_primitives() {
    assert_eq!(value("'(1 (+ 2 3))"), "(1 (+ 2 3))");
    assert_eq!(value("(cons 1 (rest (list 9 2 3)))"), "(1 2 3)");
    assert_eq!(value("(length (list 1 2))"), "2");
    assert_eq!(value("(rest ())"), "()");
    assert_eq!(value("(if () 1 2)"), "2");
}

#[test]
fn globals_persist_across_runs() {
    let mut vm = Interpreter::new();
    assert_eq!(vm.run("(set x 5) (+ x 2)").unwrap().value, "7");
    assert_eq!(vm.run("x").unwrap().value, "5");
}

#[test]
fn recursion_has_fresh_locals() {
    assert_eq!(value("(function sum-down (n) (define saved n) (if (= n 0) 0 (+ (sum-down (- n 1)) saved))) (sum-down 10)"),"55");
}

#[test]
fn lexical_not_dynamic_scope() {
    assert_eq!(value("(function make-adder (x) (lambda (y) (+ x y))) (set add-ten (make-adder 10)) (function caller (x) (add-ten 7)) (caller 100)"),"17");
}

#[test]
fn set_updates_nearest_binding_define_stays_local() {
    assert_eq!(value("(set x 1) (function change () (set x 2)) (change) (function local () (define x 99) (set x 100)) (local) x"),"2");
}

#[test]
fn mutable_closure_survives_between_runs() {
    let mut vm = Interpreter::new();
    vm.run("(function make-counter () (define x 0) (lambda () (set x (+ x 1)))) (set counter (make-counter))").unwrap();
    assert_eq!(vm.run("(counter)").unwrap().value, "1");
    assert_eq!(vm.run("(counter)").unwrap().value, "2");
    assert!(vm.live_environments() <= 3);
}

#[test]
fn loops_return_last_body_value() {
    assert_eq!(value("(while false (output 1))"), "nil");
    assert_eq!(value("(set i 0) (while (< i 3) (set i (+ i 1)))"), "3");
}

#[test]
fn garbage_collection_reclaims_calls_and_preserves_reachable_closures() {
    let mut vm = Interpreter::new();
    vm.max_steps = 500_000;
    let result=vm.run("(function make-adder (x) (lambda (y) (+ x y))) (set add (make-adder 5)) (function f (x) (+ x 1)) (set i 0) (while (< i 3000) (f i) (set i (+ i 1))) (add 7)").unwrap();
    assert_eq!(result.value, "12");
    assert!(vm.live_environments() <= 3);
    assert_eq!(vm.run("(add 2)").unwrap().value, "7");
}

#[test]
fn garbage_collection_roots_operand_stack_and_list_closures() {
    let mut vm = Interpreter::new();
    vm.max_steps = 500_000;
    assert_eq!(vm.run("(function make (x) (lambda () x)) (function churn () (define i 0) (while (< i 2000) (make i) (set i (+ i 1))) 9) (set saved (list (make 7))) (list (make 8) (churn)) ((first saved))").unwrap().value,"7");
    assert_eq!(
        vm.run("((first (list (make 8) (churn))))").unwrap().value,
        "8"
    );
}

#[test]
fn call_stack_is_explicit_and_bounded() {
    let mut vm = Interpreter::new();
    vm.max_call_depth = 1500;
    assert_eq!(
        vm.run("(function down (n) (if (= n 0) 0 (down (- n 1)))) (down 1200)")
            .unwrap()
            .value,
        "0"
    );
    vm.max_call_depth = 10;
    assert!(vm
        .run("(down 20)")
        .unwrap_err()
        .message
        .contains("call depth"));
    assert_eq!(vm.run("(+ 1 2)").unwrap().value, "3");
}

#[test]
fn step_limit_and_partial_output() {
    let mut vm = Interpreter::new();
    vm.max_steps = 30;
    assert!(vm
        .run("(output 7) (while true (block))")
        .unwrap_err()
        .message
        .contains("step limit"));
    assert_eq!(vm.output(), ["7"]);
}

#[test]
fn runtime_errors_have_source_positions() {
    for source in [
        "(/ 1 0)",
        "(+ \"text\" 1)",
        "(+ true 1)",
        "(first (list))",
        "(1 2)",
        "(first 1 2)",
    ] {
        let error = Interpreter::new().run(source).unwrap_err();
        assert_eq!(error.line, 1);
        assert!(error.column >= 1, "{source}");
    }
    let error = Interpreter::new().run("\n  missing").unwrap_err();
    assert_eq!((error.line, error.column), (2, 3));
    assert!(error.message.contains("undefined name"));
}

#[test]
fn compiler_validates_special_forms_and_parameters() {
    for source in [
        "(if)",
        "(set 123 4)",
        "(lambda (x x) x)",
        "(function f x 1)",
        "(quote)",
    ] {
        assert!(Interpreter::new().run(source).is_err(), "{source}");
    }
    assert!(Interpreter::new()
        .run("(function f (x) x) (f)")
        .unwrap_err()
        .message
        .contains("expects 1 arguments"));
}

#[test]
fn incomplete_input_is_distinct_from_bad_input() {
    for source in ["(", "(+ 1", "'", "\"unfinished"] {
        assert!(inspect_ast(source).unwrap_err().incomplete, "{source}");
    }
    assert!(!inspect_ast(")").unwrap_err().incomplete);
    assert!(inspect_ast(r#""bad\q""#).is_err());
    assert!(
        inspect_ast(&format!("{}1{}", "(".repeat(160), ")".repeat(160)))
            .unwrap_err()
            .message
            .contains("nesting limit")
    );
}

#[test]
fn overflow_reports_error_instead_of_wrapping() {
    for source in [
        "9223372036854775808",
        "(+ 9223372036854775807 1)",
        "(* 9223372036854775807 2)",
        "(- -9223372036854775808)",
        "(// -9223372036854775808 -1)",
    ] {
        assert!(Interpreter::new().run(source).is_err(), "{source}");
    }
}

#[test]
fn trace_records_calls_returns_changes_and_errors() {
    let mut vm = Interpreter::new();
    vm.tracing = true;
    assert_eq!(
        vm.run("(function inc (x) (set x (+ x 1))) (inc 4)")
            .unwrap()
            .value,
        "5"
    );
    let call = vm.trace().iter().find(|e| e["event"] == "call").unwrap();
    let change = vm.trace().iter().find(|e| e["event"] == "set").unwrap();
    assert_eq!(call["bindings"], json!({"x":"4"}));
    assert_eq!(change["before"], "4");
    assert_eq!(change["after"], "5");
    assert_eq!(call["scope"], change["scope"]);
    assert!(vm
        .trace()
        .iter()
        .any(|e| e["event"] == "return" && e["value"] == "5"));
    assert!(vm.run("(function bad () missing) (bad)").is_err());
    assert!(vm.trace().iter().any(|e| e["event"] == "error"));
}

#[test]
fn empty_program_returns_nil() {
    assert_eq!(value(" ; ignored\n # ignored"), "nil");
}
