use std::{
    fs,
    io::Write,
    path::PathBuf,
    process::{Command, Stdio},
    time::{SystemTime, UNIX_EPOCH},
};

fn command() -> Command {
    Command::new(env!("CARGO_BIN_EXE_lisp-rs"))
}
fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .to_path_buf()
}
struct TempFile(PathBuf);
impl TempFile {
    fn new(label: &str) -> Self {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        Self(std::env::temp_dir().join(format!(
            "lisp-rs-{}-{nonce}-{label}.json",
            std::process::id()
        )))
    }
}
impl Drop for TempFile {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.0);
    }
}

#[test]
fn file_has_exact_output_without_extra_value() {
    let result = command()
        .arg(root().join("examples/fibonacci.lisp"))
        .output()
        .unwrap();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    assert_eq!(
        String::from_utf8(result.stdout).unwrap(),
        "0 1\n1 1\n2 2\n3 3\n4 5\n5 8\n6 13\n7 21\n8 34\n9 55\n"
    );
}

#[test]
fn expression_ast_and_tokens() {
    let result = command().args(["-e", "(+ 1 2)"]).output().unwrap();
    assert!(result.status.success());
    assert_eq!(result.stdout, b"3\n");
    let result = command().args(["-e", "(+ 1 2)", "--ast"]).output().unwrap();
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&result.stdout).unwrap(),
        serde_json::json!([["+", 1, 2]])
    );
    let result = command()
        .args(["-e", "(+ 1 2)", "--tokens"])
        .output()
        .unwrap();
    assert!(String::from_utf8_lossy(&result.stdout).contains("EOF"));
}

#[test]
fn repl_reads_multiple_lines_and_keeps_globals() {
    let mut child = command()
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(b"(set x 4)\n(+ x\n3)\n:quit\n")
        .unwrap();
    let result = child.wait_with_output().unwrap();
    assert!(result.status.success());
    let output = String::from_utf8(result.stdout).unwrap();
    assert!(output.contains("4\n"));
    assert!(output.contains("7\n"));
}

#[test]
fn failures_save_partial_output_and_trace() {
    let trace = TempFile::new("error");
    let result = command()
        .args([
            "-e",
            "(output 7) (function bad () missing) (bad)",
            "--trace",
        ])
        .arg(&trace.0)
        .output()
        .unwrap();
    assert_eq!(result.status.code(), Some(1));
    assert_eq!(result.stdout, b"7\n");
    assert!(String::from_utf8_lossy(&result.stderr).contains("undefined name 'missing'"));
    let events: Vec<serde_json::Value> =
        serde_json::from_str(&fs::read_to_string(&trace.0).unwrap()).unwrap();
    assert!(events.iter().any(|e| e["event"] == "error"));
}

#[test]
fn invalid_options_and_missing_files_have_nonzero_status() {
    for args in [
        vec!["-e", "1", "--max-steps", "0"],
        vec!["--unknown"],
        vec!["--ast"],
        vec!["file", "-e", "1"],
    ] {
        assert_eq!(
            command().args(args).output().unwrap().status.code(),
            Some(2)
        );
    }
    assert_eq!(
        command()
            .arg("does-not-exist.lisp")
            .output()
            .unwrap()
            .status
            .code(),
        Some(1)
    );
}

#[test]
fn requested_trace_write_failure_is_reported() {
    let nonexistent = TempFile::new("missing-parent");
    let result = command()
        .args(["-e", "1", "--trace"])
        .arg(nonexistent.0.join("trace.json"))
        .output()
        .unwrap();
    assert_eq!(result.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&result.stderr).contains("could not write trace"));
}

#[test]
fn instruction_budget_stops_infinite_loop() {
    let result = command()
        .args(["-e", "(while true (block))", "--max-steps", "20"])
        .output()
        .unwrap();
    assert_eq!(result.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&result.stderr).contains("step limit"));
}
