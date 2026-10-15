use crate::syntax::{parse, Error, Expr, Kind, Position};
use serde_json::{json, Value as Json};
use std::{collections::HashMap, rc::Rc};

fn int_float_order(integer: i64, float: f64) -> std::cmp::Ordering {
    use std::cmp::Ordering;
    if float < i64::MIN as f64 {
        return Ordering::Greater;
    }
    if float >= -(i64::MIN as f64) {
        return Ordering::Less;
    }
    match integer.cmp(&(float as i64)) {
        Ordering::Equal => (integer as f64)
            .partial_cmp(&float)
            .expect("finite numeric values"),
        order => order,
    }
}

#[derive(Clone)]
enum Value {
    Nil,
    Bool(bool),
    Int(i64),
    Float(f64),
    String(Rc<str>),
    Symbol(Rc<str>),
    List(Rc<Vec<Value>>),
    Function(Rc<Function>),
    Builtin(&'static str),
}

impl Value {
    fn render(&self) -> String {
        match self {
            Self::Nil => "nil".into(),
            Self::Bool(b) => b.to_string(),
            Self::Int(n) => n.to_string(),
            Self::Float(n) => {
                if n.fract() == 0.0 {
                    format!("{n:.1}")
                } else {
                    n.to_string()
                }
            }
            Self::String(s) | Self::Symbol(s) => s.to_string(),
            Self::List(xs) => format!(
                "({})",
                xs.iter().map(Self::render).collect::<Vec<_>>().join(" ")
            ),
            Self::Function(f) => format!("<function {}>", f.template.name),
            Self::Builtin(name) => format!("<builtin {name}>"),
        }
    }
    fn truthy(&self) -> bool {
        match self {
            Self::Nil => false,
            Self::Bool(b) => *b,
            Self::Int(n) => *n != 0,
            Self::Float(n) => *n != 0.0,
            Self::String(s) | Self::Symbol(s) => !s.is_empty(),
            Self::List(xs) => !xs.is_empty(),
            _ => true,
        }
    }
    fn number(&self) -> Result<f64, String> {
        match self {
            Self::Int(n) => Ok(*n as f64),
            Self::Float(n) => Ok(*n),
            _ => Err("expected numeric arguments".into()),
        }
    }
    fn equals(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Nil, Self::Nil) => true,
            (Self::Bool(a), Self::Bool(b)) => a == b,
            (Self::Int(a), Self::Int(b)) => a == b,
            (Self::Int(a), Self::Float(b)) | (Self::Float(b), Self::Int(a)) => {
                b.is_finite()
                    && b.fract() == 0.0
                    && *b >= i64::MIN as f64
                    && *b < -(i64::MIN as f64)
                    && *a == *b as i64
            }
            (Self::Bool(a), Self::Int(b)) | (Self::Int(b), Self::Bool(a)) => i64::from(*a) == *b,
            (Self::Bool(a), Self::Float(b)) | (Self::Float(b), Self::Bool(a)) => {
                f64::from(u8::from(*a)) == *b
            }
            (Self::Float(a), Self::Float(b)) => a == b,
            (Self::String(a) | Self::Symbol(a), Self::String(b) | Self::Symbol(b)) => a == b,
            (Self::List(a), Self::List(b)) => {
                a.len() == b.len() && a.iter().zip(b.iter()).all(|(x, y)| x.equals(y))
            }
            (Self::Function(a), Self::Function(b)) => Rc::ptr_eq(a, b),
            (Self::Builtin(a), Self::Builtin(b)) => a == b,
            _ => false,
        }
    }
}

#[derive(Clone)]
enum Op {
    Constant(Value),
    Load(String),
    Store(String, bool),
    Pop,
    Dup,
    Jump(usize),
    JumpFalse(usize),
    JumpTrue(usize),
    CheckCallable,
    MakeFunction(Rc<Template>, bool),
    Call(usize),
    Return,
}
#[derive(Clone)]
struct Instruction {
    op: Op,
    pos: Position,
}
#[derive(Clone)]
struct Chunk {
    code: Vec<Instruction>,
}
struct Template {
    name: String,
    parameters: Vec<String>,
    chunk: Rc<Chunk>,
}
struct Function {
    template: Rc<Template>,
    environment: usize,
}

fn quoted(expr: &Expr) -> Value {
    match &expr.kind {
        Kind::Int(n) => Value::Int(*n),
        Kind::Float(n) => Value::Float(*n),
        Kind::String(s) => Value::String(s.as_str().into()),
        Kind::Symbol(s) => Value::Symbol(s.as_str().into()),
        Kind::List(xs) => Value::List(Rc::new(xs.iter().map(quoted).collect())),
    }
}

struct Compiler {
    code: Vec<Instruction>,
}
impl Compiler {
    fn emit(&mut self, op: Op, pos: Position) -> usize {
        self.code.push(Instruction { op, pos });
        self.code.len() - 1
    }
    fn patch(&mut self, index: usize) {
        let target = self.code.len();
        match &mut self.code[index].op {
            Op::Jump(n) | Op::JumpFalse(n) | Op::JumpTrue(n) => *n = target,
            _ => unreachable!("only compiler-generated jumps can be patched"),
        }
    }
    fn arity(expr: &Expr, args: &[Expr], min: usize, max: usize) -> Result<(), Error> {
        if args.len() < min || args.len() > max {
            Err(Error::at(
                expr.pos,
                "wrong number of arguments for special form",
            ))
        } else {
            Ok(())
        }
    }
    fn block(&mut self, expressions: &[Expr], pos: Position) -> Result<(), Error> {
        if expressions.is_empty() {
            self.emit(Op::Constant(Value::Nil), pos);
        }
        for (i, expression) in expressions.iter().enumerate() {
            self.expression(expression)?;
            if i + 1 < expressions.len() {
                self.emit(Op::Pop, expression.pos);
            }
        }
        Ok(())
    }
    fn expression(&mut self, expr: &Expr) -> Result<(), Error> {
        let pos = expr.pos;
        let items = match &expr.kind {
            Kind::Symbol(name) => {
                self.emit(Op::Load(name.clone()), pos);
                return Ok(());
            }
            Kind::List(xs) if !xs.is_empty() => xs,
            _ => {
                self.emit(Op::Constant(quoted(expr)), pos);
                return Ok(());
            }
        };
        let command = match &items[0].kind {
            Kind::Symbol(name) => name.as_str(),
            _ => "",
        };
        let args = &items[1..];
        match command {
            "quote" => {
                Self::arity(expr, args, 1, 1)?;
                self.emit(Op::Constant(quoted(&args[0])), pos);
            }
            "block" | "begin" | "progn" => self.block(args, pos)?,
            "if" => {
                Self::arity(expr, args, 2, 3)?;
                self.expression(&args[0])?;
                let otherwise = self.emit(Op::JumpFalse(0), pos);
                self.expression(&args[1])?;
                let done = self.emit(Op::Jump(0), pos);
                self.patch(otherwise);
                if args.len() == 3 {
                    self.expression(&args[2])?;
                } else {
                    self.emit(Op::Constant(Value::Nil), pos);
                }
                self.patch(done);
            }
            "and" | "or" => {
                let mut exits = Vec::new();
                if args.is_empty() {
                    self.emit(Op::Constant(Value::Bool(command == "and")), pos);
                }
                for (i, arg) in args.iter().enumerate() {
                    self.expression(arg)?;
                    if i + 1 < args.len() {
                        self.emit(Op::Dup, pos);
                        exits.push(self.emit(
                            if command == "and" {
                                Op::JumpFalse(0)
                            } else {
                                Op::JumpTrue(0)
                            },
                            pos,
                        ));
                        self.emit(Op::Pop, pos);
                    }
                }
                for exit in exits {
                    self.patch(exit);
                }
            }
            "while" => {
                Self::arity(expr, args, 2, usize::MAX)?;
                self.emit(Op::Constant(Value::Nil), pos);
                let test = self.code.len();
                self.expression(&args[0])?;
                let done = self.emit(Op::JumpFalse(0), pos);
                self.emit(Op::Pop, pos);
                self.block(&args[1..], pos)?;
                self.emit(Op::Jump(test), pos);
                self.patch(done);
            }
            "set" | "define" => {
                Self::arity(expr, args, 2, 2)?;
                let name = args[0].name()?;
                self.expression(&args[1])?;
                self.emit(Op::Store(name, command == "define"), pos);
            }
            "function" | "lambda" => {
                Self::arity(
                    expr,
                    args,
                    if command == "function" { 3 } else { 2 },
                    usize::MAX,
                )?;
                let (name, params, body) = if command == "function" {
                    (args[0].name()?, &args[1], &args[2..])
                } else {
                    ("<lambda>".into(), &args[0], &args[1..])
                };
                let parameters = match &params.kind {
                    Kind::List(xs) => xs.iter().map(Expr::name).collect::<Result<Vec<_>, _>>()?,
                    _ => {
                        return Err(Error::at(
                            params.pos,
                            "expected a parenthesized parameter list",
                        ))
                    }
                };
                let mut seen = std::collections::HashSet::new();
                if parameters.iter().any(|p| !seen.insert(p)) {
                    return Err(Error::at(params.pos, "duplicate parameter name"));
                }
                let chunk = compile(body, pos)?;
                self.emit(
                    Op::MakeFunction(
                        Rc::new(Template {
                            name,
                            parameters,
                            chunk,
                        }),
                        command == "function",
                    ),
                    pos,
                );
            }
            _ => {
                self.expression(&items[0])?;
                self.emit(Op::CheckCallable, items[0].pos);
                for arg in args {
                    self.expression(arg)?;
                }
                self.emit(Op::Call(args.len()), pos);
            }
        }
        Ok(())
    }
}
fn compile(expressions: &[Expr], pos: Position) -> Result<Rc<Chunk>, Error> {
    let mut compiler = Compiler { code: Vec::new() };
    compiler.block(expressions, pos)?;
    compiler.emit(Op::Return, pos);
    Ok(Rc::new(Chunk {
        code: compiler.code,
    }))
}

struct Environment {
    parent: Option<usize>,
    bindings: HashMap<String, Value>,
    scope: usize,
}
struct Heap {
    environments: Vec<Option<Environment>>,
    free: Vec<usize>,
    live: usize,
    next_scope: usize,
}
impl Heap {
    fn new() -> Self {
        Self {
            environments: vec![Some(Environment {
                parent: None,
                bindings: HashMap::new(),
                scope: 0,
            })],
            free: Vec::new(),
            live: 1,
            next_scope: 1,
        }
    }
    fn env(&self, id: usize) -> &Environment {
        self.environments[id].as_ref().expect("live environment")
    }
    fn env_mut(&mut self, id: usize) -> &mut Environment {
        self.environments[id].as_mut().expect("live environment")
    }
    fn allocate(&mut self, parent: usize, bindings: HashMap<String, Value>) -> usize {
        let env = Some(Environment {
            parent: Some(parent),
            bindings,
            scope: self.next_scope,
        });
        self.next_scope += 1;
        self.live += 1;
        if let Some(id) = self.free.pop() {
            self.environments[id] = env;
            id
        } else {
            self.environments.push(env);
            self.environments.len() - 1
        }
    }
    fn owner(&self, mut id: usize, name: &str) -> Option<usize> {
        loop {
            let env = self.env(id);
            if env.bindings.contains_key(name) {
                return Some(id);
            }
            id = env.parent?;
        }
    }
    // Environments are arena IDs rather than Rc cycles. Trace reachable closure
    // environments from globals, active frames and operand values, then reclaim the rest.
    fn collect(&mut self, stack: &[Value], frames: &[Frame]) {
        fn references(value: &Value, todo: &mut Vec<usize>) {
            match value {
                Value::Function(f) => todo.push(f.environment),
                Value::List(xs) => {
                    for x in xs.iter() {
                        references(x, todo);
                    }
                }
                _ => {}
            }
        }
        let mut marked = vec![false; self.environments.len()];
        let mut todo = vec![0];
        todo.extend(frames.iter().map(|frame| frame.environment));
        for value in stack {
            references(value, &mut todo);
        }
        while let Some(id) = todo.pop() {
            if marked[id] {
                continue;
            }
            marked[id] = true;
            let env = self.env(id);
            if let Some(parent) = env.parent {
                todo.push(parent);
            }
            for value in env.bindings.values() {
                references(value, &mut todo);
            }
        }
        for (id, marked) in marked.iter().enumerate() {
            if !marked && self.environments[id].take().is_some() {
                self.free.push(id);
                self.live -= 1;
            }
        }
    }
}

struct Frame {
    chunk: Rc<Chunk>,
    ip: usize,
    environment: usize,
    base: usize,
    name: Option<String>,
    call_pos: Position,
}

/// A completed run. Output is captured for embedding in another Rust application.
#[derive(Debug)]
pub struct Execution {
    pub value: String,
    pub is_nil: bool,
    pub output: Vec<String>,
    pub steps: usize,
}

/// Persistent globals plus a bytecode VM; each run has a fresh execution budget.
pub struct Interpreter {
    heap: Heap,
    pub max_steps: usize,
    pub max_call_depth: usize,
    pub tracing: bool,
    output: Vec<String>,
    events: Vec<Json>,
    steps: usize,
    collection_threshold: usize,
}
impl Default for Interpreter {
    fn default() -> Self {
        Self::new()
    }
}
impl Interpreter {
    pub fn new() -> Self {
        let mut heap = Heap::new();
        for name in [
            "+", "-", "*", "/", "//", "%", "<", "<=", ">", ">=", "=", "!=", "not", "list", "first",
            "rest", "length", "cons", "output",
        ] {
            heap.env_mut(0)
                .bindings
                .insert(name.into(), Value::Builtin(name));
        }
        heap.env_mut(0).bindings.extend([
            ("true".into(), Value::Bool(true)),
            ("false".into(), Value::Bool(false)),
            ("nil".into(), Value::Nil),
        ]);
        Self {
            heap,
            max_steps: 1_000_000,
            max_call_depth: 1000,
            tracing: false,
            output: Vec::new(),
            events: Vec::new(),
            steps: 0,
            collection_threshold: 1024,
        }
    }
    pub fn trace(&self) -> &[Json] {
        &self.events
    }
    /// Partial output remains available after an error.
    pub fn output(&self) -> &[String] {
        &self.output
    }
    pub fn live_environments(&self) -> usize {
        self.heap.live
    }

    pub fn run(&mut self, source: &str) -> Result<Execution, Error> {
        self.output.clear();
        self.events.clear();
        self.steps = 0;
        let pos = Position { line: 1, column: 1 };
        let program = compile(&parse(source)?, pos)?;
        let result = self.execute(program);
        // No environment IDs escape through Execution's rendered strings.
        self.heap.collect(&[], &[]);
        self.collection_threshold = self.heap.live.saturating_mul(2).saturating_add(1024);
        result.map(|value| Execution {
            value: value.render(),
            is_nil: matches!(value, Value::Nil),
            output: self.output.clone(),
            steps: self.steps,
        })
    }

    fn event(&mut self, kind: &str, pos: Position, env: usize, frames: &[Frame], extra: Json) {
        if !self.tracing {
            return;
        }
        let stack: Vec<_> = frames.iter().filter_map(|f| f.name.clone()).collect();
        let scope = self.heap.env(env).scope;
        let mut record = json!({"event":kind,"step":self.steps,"line":pos.line,"column":pos.column,"scope":scope,"stack":stack});
        if let (Some(record), Some(extra)) = (record.as_object_mut(), extra.as_object()) {
            record.extend(extra.clone());
        }
        self.events.push(record);
    }

    fn execute(&mut self, chunk: Rc<Chunk>) -> Result<Value, Error> {
        let mut stack = Vec::new();
        let mut frames = vec![Frame {
            chunk,
            ip: 0,
            environment: 0,
            base: 0,
            name: None,
            call_pos: Position { line: 1, column: 1 },
        }];
        loop {
            let frame = frames.last_mut().expect("active frame");
            let instruction = frame.chunk.code[frame.ip].clone();
            frame.ip += 1;
            let pos = instruction.pos;
            let env = frame.environment;
            self.steps += 1;
            if self.steps > self.max_steps {
                self.event(
                    "error",
                    pos,
                    env,
                    &frames,
                    json!({"message":"execution step limit exceeded"}),
                );
                return Err(Error::at(
                    pos,
                    format!("execution step limit exceeded ({})", self.max_steps),
                ));
            }
            self.event("step", pos, env, &frames, json!({}));
            let step = (|| -> Result<Option<Value>, Error> {
                match instruction.op {
                    Op::Constant(value) => stack.push(value),
                    Op::Load(name) => {
                        let owner = self
                            .heap
                            .owner(env, &name)
                            .ok_or_else(|| Error::at(pos, format!("undefined name '{name}'")))?;
                        stack.push(self.heap.env(owner).bindings[&name].clone());
                    }
                    Op::Store(name, define) => {
                        let value = stack.last().expect("compiled value").clone();
                        let owner = if define {
                            env
                        } else {
                            self.heap.owner(env, &name).unwrap_or(env)
                        };
                        if self.tracing {
                            let before = self
                                .heap
                                .env(owner)
                                .bindings
                                .get(&name)
                                .map(Value::render)
                                .unwrap_or("<unbound>".into());
                            self.event(
                                "set",
                                pos,
                                owner,
                                &frames,
                                json!({"name":name,"before":before,"after":value.render()}),
                            );
                        }
                        self.heap.env_mut(owner).bindings.insert(name, value);
                    }
                    Op::Pop => {
                        stack.pop().expect("compiled operand");
                    }
                    Op::Dup => stack.push(stack.last().expect("compiled operand").clone()),
                    Op::Jump(target) => frames.last_mut().unwrap().ip = target,
                    Op::JumpFalse(target) => {
                        if !stack.pop().unwrap().truthy() {
                            frames.last_mut().unwrap().ip = target;
                        }
                    }
                    Op::JumpTrue(target) => {
                        if stack.pop().unwrap().truthy() {
                            frames.last_mut().unwrap().ip = target;
                        }
                    }
                    Op::CheckCallable => {
                        if !matches!(stack.last(), Some(Value::Function(_) | Value::Builtin(_))) {
                            return Err(Error::at(pos, "attempted to call a non-function"));
                        }
                    }
                    Op::MakeFunction(template, bind) => {
                        let value = Value::Function(Rc::new(Function {
                            template: template.clone(),
                            environment: env,
                        }));
                        if bind {
                            self.heap
                                .env_mut(env)
                                .bindings
                                .insert(template.name.clone(), value.clone());
                        }
                        stack.push(value);
                    }
                    Op::Call(count) => {
                        let arguments = stack.split_off(stack.len() - count);
                        let callable = stack.pop().expect("compiled callable");
                        match callable {
                            Value::Builtin(name) => stack.push(
                                self.builtin(name, &arguments)
                                    .map_err(|e| Error::at(pos, e))?,
                            ),
                            Value::Function(function) => {
                                let template = &function.template;
                                if arguments.len() != template.parameters.len() {
                                    return Err(Error::at(
                                        pos,
                                        format!(
                                            "{} expects {} arguments; got {}",
                                            template.name,
                                            template.parameters.len(),
                                            arguments.len()
                                        ),
                                    ));
                                }
                                if frames.len() > self.max_call_depth {
                                    return Err(Error::at(
                                        pos,
                                        format!(
                                            "function call depth limit exceeded ({})",
                                            self.max_call_depth
                                        ),
                                    ));
                                }
                                let bindings: HashMap<_, _> =
                                    template.parameters.iter().cloned().zip(arguments).collect();
                                let env = self.heap.allocate(function.environment, bindings);
                                frames.push(Frame {
                                    chunk: template.chunk.clone(),
                                    ip: 0,
                                    environment: env,
                                    base: stack.len(),
                                    name: Some(template.name.clone()),
                                    call_pos: pos,
                                });
                                if self.tracing {
                                    let bindings: HashMap<_, _> = self
                                        .heap
                                        .env(env)
                                        .bindings
                                        .iter()
                                        .map(|(k, v)| (k.clone(), v.render()))
                                        .collect();
                                    self.event(
                                        "call",
                                        pos,
                                        env,
                                        &frames,
                                        json!({"function":template.name,"bindings":bindings}),
                                    );
                                }
                            }
                            _ => return Err(Error::at(pos, "attempted to call a non-function")),
                        }
                    }
                    Op::Return => {
                        let value = stack.pop().expect("compiled return value");
                        let frame = frames.last().unwrap();
                        if let Some(name) = &frame.name {
                            self.event(
                                "return",
                                frame.call_pos,
                                env,
                                &frames,
                                json!({"function":name,"value":value.render()}),
                            );
                        }
                        let frame = frames.pop().unwrap();
                        stack.truncate(frame.base);
                        if frames.is_empty() {
                            return Ok(Some(value));
                        }
                        stack.push(value);
                    }
                }
                Ok(None)
            })();
            match step {
                Ok(Some(value)) => return Ok(value),
                Ok(None) => {}
                Err(error) => {
                    self.event(
                        "error",
                        pos,
                        env,
                        &frames,
                        json!({"message":error.to_string()}),
                    );
                    return Err(error);
                }
            }
            if self.heap.live >= self.collection_threshold {
                self.heap.collect(&stack, &frames);
                self.collection_threshold = self.heap.live.saturating_mul(2).saturating_add(1024);
            }
        }
    }

    fn builtin(&mut self, name: &str, args: &[Value]) -> Result<Value, String> {
        let (minimum, maximum) = match name {
            "+" | "*" | "list" | "output" => (0, usize::MAX),
            "-" | "/" => (1, usize::MAX),
            "<" | "<=" | ">" | ">=" | "=" => (2, usize::MAX),
            "not" | "first" | "rest" | "length" => (1, 1),
            _ => (2, 2),
        };
        if args.len() < minimum || args.len() > maximum {
            return Err(format!("wrong number of arguments for {name}"));
        }
        match name {
            "+" | "-" | "*" | "/" | "//" | "%" => arithmetic(name, args),
            "<" | "<=" | ">" | ">=" => {
                for value in args {
                    value.number()?;
                }
                let result = args.windows(2).all(|pair| {
                    let ordering = match (&pair[0], &pair[1]) {
                        (Value::Int(a), Value::Int(b)) => a.partial_cmp(b),
                        (Value::Int(a), Value::Float(b)) => Some(int_float_order(*a, *b)),
                        (Value::Float(a), Value::Int(b)) => Some(int_float_order(*b, *a).reverse()),
                        _ => pair[0]
                            .number()
                            .unwrap()
                            .partial_cmp(&pair[1].number().unwrap()),
                    };
                    use std::cmp::Ordering::{Equal, Greater, Less};
                    match name {
                        "<" => ordering == Some(Less),
                        "<=" => matches!(ordering, Some(Less | Equal)),
                        ">" => ordering == Some(Greater),
                        _ => matches!(ordering, Some(Greater | Equal)),
                    }
                });
                Ok(Value::Bool(result))
            }
            "=" => Ok(Value::Bool(
                args.windows(2).all(|pair| pair[0].equals(&pair[1])),
            )),
            "!=" => Ok(Value::Bool(!args[0].equals(&args[1]))),
            "not" => Ok(Value::Bool(!args[0].truthy())),
            "list" => Ok(Value::List(Rc::new(args.to_vec()))),
            "first" => match &args[0] {
                Value::List(xs) => xs
                    .first()
                    .cloned()
                    .ok_or("first requires a nonempty list".into()),
                Value::String(s) | Value::Symbol(s) => s
                    .chars()
                    .next()
                    .map(|c| Value::String(c.to_string().into()))
                    .ok_or("first requires a nonempty string".into()),
                _ => Err("first expects a list or string".into()),
            },
            "rest" => match &args[0] {
                Value::List(xs) => Ok(Value::List(Rc::new(xs.iter().skip(1).cloned().collect()))),
                Value::String(s) | Value::Symbol(s) => {
                    Ok(Value::String(s.chars().skip(1).collect::<String>().into()))
                }
                _ => Err("rest expects a list or string".into()),
            },
            "length" => match &args[0] {
                Value::List(xs) => Ok(Value::Int(xs.len() as i64)),
                Value::String(s) | Value::Symbol(s) => Ok(Value::Int(s.chars().count() as i64)),
                _ => Err("length expects a list or string".into()),
            },
            "cons" => match &args[1] {
                Value::List(xs) => {
                    let mut list = Vec::with_capacity(xs.len() + 1);
                    list.push(args[0].clone());
                    list.extend(xs.iter().cloned());
                    Ok(Value::List(Rc::new(list)))
                }
                _ => Err("cons expects a list as its second argument".into()),
            },
            "output" => {
                self.output
                    .push(args.iter().map(Value::render).collect::<Vec<_>>().join(" "));
                Ok(Value::Nil)
            }
            _ => unreachable!("builtins are installed by the VM"),
        }
    }
}

fn arithmetic(name: &str, args: &[Value]) -> Result<Value, String> {
    for value in args {
        value.number()?;
    }
    let overflow = || "signed 64-bit integer overflow".to_string();
    if name != "/" && args.iter().all(|v| matches!(v, Value::Int(_))) {
        let xs: Vec<i64> = args
            .iter()
            .map(|v| {
                if let Value::Int(n) = v {
                    *n
                } else {
                    unreachable!()
                }
            })
            .collect();
        let result = match name {
            "+" => xs
                .iter()
                .try_fold(0i64, |a, b| a.checked_add(*b).ok_or_else(overflow))?,
            "*" => xs
                .iter()
                .try_fold(1i64, |a, b| a.checked_mul(*b).ok_or_else(overflow))?,
            "-" if xs.len() == 1 => xs[0].checked_neg().ok_or_else(overflow)?,
            "-" => xs[1..]
                .iter()
                .try_fold(xs[0], |a, b| a.checked_sub(*b).ok_or_else(overflow))?,
            "//" | "%" => {
                if xs[1] == 0 {
                    return Err("division by zero".into());
                }
                let quotient = xs[0].checked_div(xs[1]).ok_or_else(overflow)?;
                let remainder = xs[0].checked_rem(xs[1]).ok_or_else(overflow)?;
                let adjust = remainder != 0 && (remainder < 0) != (xs[1] < 0);
                if name == "//" {
                    if adjust {
                        quotient.checked_sub(1).ok_or_else(overflow)?
                    } else {
                        quotient
                    }
                } else if adjust {
                    remainder.checked_add(xs[1]).ok_or_else(overflow)?
                } else {
                    remainder
                }
            }
            _ => unreachable!(),
        };
        return Ok(Value::Int(result));
    }
    let xs: Vec<f64> = args.iter().map(|v| v.number().unwrap()).collect();
    let result = match name {
        "+" => xs.iter().sum(),
        "*" => xs.iter().product(),
        "-" if xs.len() == 1 => -xs[0],
        "-" => xs[1..].iter().fold(xs[0], |a, b| a - b),
        "/" => {
            let (start, denominators) = if xs.len() == 1 {
                (1.0, &xs[..])
            } else {
                (xs[0], &xs[1..])
            };
            if denominators.contains(&0.0) {
                return Err("division by zero".into());
            }
            denominators.iter().fold(start, |a, b| a / b)
        }
        "//" | "%" => {
            if xs[1] == 0.0 {
                return Err("division by zero".into());
            }
            let quotient = (xs[0] / xs[1]).floor();
            if name == "//" {
                quotient
            } else {
                xs[0] - quotient * xs[1]
            }
        }
        _ => unreachable!(),
    };
    if !result.is_finite() {
        return Err("floating-point result out of range".into());
    }
    Ok(Value::Float(result))
}
