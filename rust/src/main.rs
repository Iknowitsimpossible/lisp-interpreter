use lisp_interpreter::{inspect_ast, inspect_tokens, Interpreter};
use std::{
    env, fs,
    io::{self, Write},
    path::PathBuf,
    process::ExitCode,
};

const HELP: &str = "lisp-rs [FILE | -e SOURCE] [OPTIONS]

Run a Lisp file, evaluate an expression, or start a multiline REPL.

  -e, --expression SOURCE  Evaluate source text and print its last value
  --tokens                Print positioned tokens without executing
  --ast                   Print nested parse trees without executing
  --trace PATH            Write JSON execution events (including partial failures)
  --max-steps N           Instruction budget per run (default: 1000000)
  --max-call-depth N      Maximum nested function calls (default: 1000)
  -h, --help              Show this help
";

#[derive(Default)]
struct Options {
    file: Option<PathBuf>,
    expression: Option<String>,
    tokens: bool,
    ast: bool,
    trace: Option<PathBuf>,
    max_steps: Option<usize>,
    max_call_depth: Option<usize>,
    help: bool,
}

fn options() -> Result<Options, String> {
    let mut options = Options::default();
    let mut args = env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "-h" | "--help" => options.help = true,
            "-e" | "--expression" => {
                if options.expression.is_some() {
                    return Err("provide -e only once".into());
                }
                options.expression = Some(args.next().ok_or("-e requires source text")?);
            }
            "--tokens" => options.tokens = true,
            "--ast" => options.ast = true,
            "--trace" => options.trace = Some(args.next().ok_or("--trace requires a path")?.into()),
            "--max-steps" | "--max-call-depth" => {
                let n: usize = args
                    .next()
                    .ok_or(format!("{arg} requires a number"))?
                    .parse()
                    .map_err(|_| format!("{arg} requires a positive integer"))?;
                if n == 0 {
                    return Err(format!("{arg} must be positive"));
                }
                if arg == "--max-steps" {
                    options.max_steps = Some(n);
                } else {
                    options.max_call_depth = Some(n);
                }
            }
            _ if arg.starts_with('-') => return Err(format!("unknown option: {arg}")),
            _ if options.file.is_none() => options.file = Some(arg.into()),
            _ => return Err("provide only one source file".into()),
        }
    }
    if options.file.is_some() && options.expression.is_some() {
        return Err("choose either a file or -e".into());
    }
    if options.file.is_none()
        && options.expression.is_none()
        && (options.tokens || options.ast || options.trace.is_some())
    {
        return Err("--tokens, --ast and --trace require a file or -e".into());
    }
    Ok(options)
}

fn repl(vm: &mut Interpreter) -> Result<(), String> {
    println!("Lisp Rust. Enter :quit to exit. Multiline expressions are supported.");
    let mut pending = String::new();
    loop {
        print!("{}", if pending.is_empty() { "lisp> " } else { "... " });
        io::stdout().flush().map_err(|e| e.to_string())?;
        let mut line = String::new();
        if io::stdin()
            .read_line(&mut line)
            .map_err(|e| e.to_string())?
            == 0
        {
            if !pending.is_empty() {
                eprintln!("Incomplete input discarded.");
            }
            return Ok(());
        }
        if pending.is_empty() && line.trim() == ":quit" {
            return Ok(());
        }
        pending.push_str(&line);
        match inspect_ast(&pending) {
            Err(error) if error.incomplete => continue,
            Err(error) => {
                eprintln!("Error: {error}");
                pending.clear();
                continue;
            }
            Ok(_) => {}
        }
        match vm.run(&pending) {
            Ok(result) => {
                for line in result.output {
                    println!("{line}");
                }
                if !result.is_nil {
                    println!("{}", result.value);
                }
            }
            Err(error) => {
                for line in vm.output() {
                    println!("{line}");
                }
                eprintln!("Error: {error}");
            }
        }
        pending.clear();
    }
}

fn run(options: Options) -> Result<(), String> {
    if options.help {
        println!("{HELP}");
        return Ok(());
    }
    let mut vm = Interpreter::new();
    if let Some(n) = options.max_steps {
        vm.max_steps = n;
    }
    if let Some(n) = options.max_call_depth {
        vm.max_call_depth = n;
    }
    vm.tracing = options.trace.is_some();
    let source = match (&options.file, &options.expression) {
        (Some(file), _) => {
            fs::read_to_string(file).map_err(|e| format!("{}: {e}", file.display()))?
        }
        (_, Some(code)) => code.clone(),
        _ => return repl(&mut vm),
    };
    let outcome = if options.tokens || options.ast {
        let inspection = if options.tokens {
            inspect_tokens(&source)
        } else {
            inspect_ast(&source)
        };
        inspection
            .map(|value| println!("{}", serde_json::to_string_pretty(&value).unwrap()))
            .map_err(|e| e.to_string())
    } else {
        let result = vm.run(&source);
        for line in vm.output() {
            println!("{line}");
        }
        result
            .map(|result| {
                if options.expression.is_some() && !result.is_nil {
                    println!("{}", result.value);
                }
            })
            .map_err(|e| e.to_string())
    };
    if let Some(path) = options.trace {
        let data = serde_json::to_string_pretty(vm.trace()).map_err(|e| e.to_string())?;
        fs::write(&path, data + "\n")
            .map_err(|e| format!("could not write trace {}: {e}", path.display()))?;
    }
    outcome
}

fn main() -> ExitCode {
    let options = match options() {
        Ok(options) => options,
        Err(error) => {
            eprintln!("Error: {error}\n\n{HELP}");
            return ExitCode::from(2);
        }
    };
    match run(options) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("Error: {error}");
            ExitCode::FAILURE
        }
    }
}
