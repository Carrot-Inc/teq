//! The program a session is driven over: files, definitions and bodies as data, rendered to
//! Scala. A rule changes the data and the files are written from it, so every choice of a rule
//! is an index into what exists at that moment, and a history keeps its meaning when the
//! shrinker takes out the step that made a definition.
//!
//! The programs are well typed by construction: an expression is adapted to the type its
//! place expects (`.toString`, `.length`), and a reference to something that is gone renders
//! as a literal. Errors are injected as such and taken out again by the repair rule.

use std::collections::BTreeMap;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Ty {
    Int,
    Str,
}

impl Ty {
    fn name(self) -> &'static str {
        match self {
            Ty::Int => "Int",
            Ty::Str => "String",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Macro {
    Count,
    Shape,
    Words,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Expr {
    Int(i32),
    Str(String),
    Param(usize),
    Local(usize),
    Seed,
    Call { def: usize, seed: i32, args: Vec<Expr> },
    Macro(Macro, String),
    Add(Ty, Box<Expr>, Box<Expr>),
}

#[derive(Clone, Debug, PartialEq)]
pub enum Stmt {
    Val(usize, Expr),
    Print(Expr),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Def,
    Inline,
    Val,
    Main,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Owner {
    Top,
    Object,
    Class,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Param {
    pub ty: Ty,
    pub default: Option<Expr>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Fault {
    /// The body's result is a value of the other type; needs a declared result type.
    Mismatch,
    /// The body's result names something that does not exist.
    Unknown,
    /// The declared result type names a type that does not exist.
    Signature,
}

/// What an edit may change of a definition; its id, owner and kind stay.
#[derive(Clone, Debug, PartialEq)]
pub struct Shape {
    pub params: Vec<Param>,
    pub result: Ty,
    pub declared: bool,
    pub statements: Vec<Stmt>,
    pub value: Expr,
}

#[derive(Clone, Debug)]
pub struct Def {
    pub id: usize,
    pub file: usize,
    pub owner: Owner,
    pub kind: Kind,
    pub shape: Shape,
    pub previous: Option<Shape>,
    pub fault: Option<Fault>,
}

#[derive(Clone, Debug)]
pub struct File {
    pub id: usize,
    pub package: usize,
    pub bad_import: bool,
}

impl File {
    pub fn name(&self) -> String {
        if self.id == 0 {
            "Main.scala".to_string()
        } else {
            format!("f{}.scala", self.id)
        }
    }

    /// The file with its package clause and import alone: what stands in the place of a
    /// removed file for the build that takes its definitions out before the file goes.
    pub fn emptied(&self) -> String {
        format!("package {}\n\nimport mac.*\n", self.package_name())
    }

    fn package_name(&self) -> String {
        if self.id == 0 {
            "app".to_string()
        } else {
            format!("p{}", self.package)
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct Program {
    pub files: Vec<File>,
    pub defs: Vec<Def>,
    pub next_file: usize,
    pub next_def: usize,
    pub next_local: usize,
}

/// The macros every program has, after tests/split/retype: their results depend on their
/// arguments alone, and no quote makes a class. What their runs read stands in a file of its
/// own, as in the fixture.
pub const MACROS: &str = r#"package mac

import scala.quoted.*

object Macros:
  def countImpl(text: Expr[String])(using Quotes): Expr[Int] =
    Expr(new Tally(text.valueOrAbort).words.count(Rules.longerThan(0).keeps))

  def shapeImpl(text: Expr[String])(using Quotes): Expr[String] =
    val tally = new Tally(text.valueOrAbort)
    val groups = tally.words.groupBy(_.length).toList.sortBy(_._1).map((n, ws) => n.toString + ":" + ws.length)
    Expr(groups.mkString(separator) + separator + tally.longest)

  def wordsImpl(sc: Expr[StringContext])(using Quotes): Expr[Int] =
    import quotes.reflect.*
    val parts = sc match
      case '{ StringContext(${ Varargs(ps) }*) } => ps.map(_.valueOrAbort)
      case _ => report.errorAndAbort("words needs a literal", sc)
    Expr(new Tally(parts.mkString(" ")).size)

inline def count(inline text: String): Int = ${ Macros.countImpl('text) }

inline def shape(inline text: String): String = ${ Macros.shapeImpl('text) }

extension (inline sc: StringContext)
  inline def words(inline args: Any*): Int = ${ Macros.wordsImpl('sc) }
"#;

pub const BASE: &str = r#"package mac

trait Rule:
  def keeps(word: String): Boolean

object Rules:
  def longerThan(n: Int): Rule = new Rule:
    def keeps(word: String): Boolean = word.length > n

final class Tally(text: String):
  val words: List[String] = text.split(" ").toList.filter(_.nonEmpty)
  lazy val longest: Int = words.map(_.length).foldLeft(0)((a, b) => if a > b then a else b)
  def size: Int = words.length

val separator: String = "/"
"#;

pub const MACROS_FILE: &str = "macros.scala";
/// The file of what the macros read. Its name comes first in the order of the inputs unless the
/// known defect `macro-val-order` is asked for (`known.rs`).
pub fn base_file() -> &'static str {
    if crate::known::asked("macro-val-order") {
        "base.scala"
    } else {
        "Base.scala"
    }
}

impl Program {
    /// A program of the entry file alone.
    pub fn new() -> Program {
        let mut program = Program { next_file: 1, ..Program::default() };
        program.files.push(File { id: 0, package: 0, bad_import: false });
        let shape = Shape { params: Vec::new(), result: Ty::Int, declared: true, statements: Vec::new(), value: Expr::Int(0) };
        program.defs.push(Def { id: 0, file: 0, owner: Owner::Object, kind: Kind::Main, shape, previous: None, fault: None });
        program.next_def = 1;
        program
    }

    pub fn add_file(&mut self, package: usize) -> usize {
        let id = self.next_file;
        self.next_file += 1;
        self.files.push(File { id, package, bad_import: false });
        id
    }

    /// Adds a definition and the statement of the entry point that prints it.
    pub fn add_def(&mut self, file: usize, owner: Owner, kind: Kind, shape: Shape) -> usize {
        let id = self.next_def;
        self.next_def += 1;
        self.defs.push(Def { id, file, owner, kind, shape, previous: None, fault: None });
        let call = Expr::Call { def: id, seed: 1, args: Vec::new() };
        self.def_mut(0).shape.statements.push(Stmt::Print(call));
        id
    }

    pub fn remove_def(&mut self, id: usize) {
        self.defs.retain(|def| def.id != id);
    }

    pub fn remove_file(&mut self, id: usize) {
        self.files.retain(|file| file.id != id);
        self.defs.retain(|def| def.file != id);
    }

    pub fn def(&self, id: usize) -> Option<&Def> {
        self.defs.iter().find(|def| def.id == id)
    }

    pub fn def_mut(&mut self, id: usize) -> &mut Def {
        self.defs.iter_mut().find(|def| def.id == id).unwrap()
    }

    pub fn file(&self, id: usize) -> &File {
        self.files.iter().find(|file| file.id == id).unwrap()
    }

    pub fn fresh_local(&mut self) -> usize {
        self.next_local += 1;
        self.next_local
    }

    /// Whether an injected fault stands anywhere in the program.
    pub fn faulty(&self) -> bool {
        self.defs.iter().any(|def| self.shown_fault(def).is_some()) || self.files.iter().any(|file| file.bad_import)
    }

    /// The definition's fault as the program shows it: an inline method's broken signature
    /// is withdrawn while nothing calls the method (`known`, `inline-signature-uncalled`).
    pub fn shown_fault(&self, def: &Def) -> Option<Fault> {
        match def.fault {
            Some(Fault::Signature) if def.kind == Kind::Inline && !crate::known::asked("inline-signature-uncalled") => {
                self.called(def.id).then_some(Fault::Signature)
            }
            fault => fault,
        }
    }

    /// Whether a body or a default argument of the program calls the definition.
    pub fn called(&self, id: usize) -> bool {
        fn has(e: &Expr, id: usize) -> bool {
            match e {
                Expr::Call { def, args, .. } => *def == id || args.iter().any(|arg| has(arg, id)),
                Expr::Add(_, a, b) => has(a, id) || has(b, id),
                _ => false,
            }
        }
        self.defs.iter().any(|def| {
            has(&def.shape.value, id)
                || def.shape.statements.iter().any(|s| has(statement_value(s), id))
                || def.shape.params.iter().any(|param| param.default.as_ref().is_some_and(|e| has(e, id)))
        })
    }

    /// Whether the file has a macro's call in one of its bodies.
    pub fn calls_macro(&self, file: usize) -> bool {
        fn has(e: &Expr) -> bool {
            match e {
                Expr::Macro(..) => true,
                Expr::Call { args, .. } => args.iter().any(has),
                Expr::Add(_, a, b) => has(a) || has(b),
                _ => false,
            }
        }
        self.defs.iter().filter(|def| def.file == file).any(|def| {
            has(&def.shape.value)
                || def.shape.statements.iter().any(|s| match s {
                    Stmt::Val(_, e) | Stmt::Print(e) => has(e),
                })
        })
    }

    /// The text of every file by name, the fixed ones included.
    pub fn render(&self) -> BTreeMap<String, String> {
        let mut texts = BTreeMap::new();
        texts.insert(MACROS_FILE.to_string(), MACROS.to_string());
        texts.insert(base_file().to_string(), BASE.to_string());
        for file in &self.files {
            texts.insert(file.name(), self.render_file(file));
        }
        texts
    }

    fn render_file(&self, file: &File) -> String {
        let mut text = format!("package {}\n\nimport mac.*\n", file.package_name());
        if file.bad_import {
            text.push_str(&format!("import nowhere.Thing{}\n", file.id));
        }
        let of = |owner: Owner| self.defs.iter().filter(move |def| def.file == file.id && def.owner == owner);
        for def in of(Owner::Top) {
            text.push('\n');
            self.render_def(def, "", &mut text);
        }
        if of(Owner::Object).next().is_some() {
            let name = if file.id == 0 { "Main".to_string() } else { format!("O{}", file.id) };
            text.push_str(&format!("\nobject {name}:\n"));
            for def in of(Owner::Object) {
                self.render_def(def, "  ", &mut text);
            }
        }
        if of(Owner::Class).next().is_some() {
            text.push_str(&format!("\nclass C{}(seed: Int):\n", file.id));
            for def in of(Owner::Class) {
                self.render_def(def, "  ", &mut text);
            }
        }
        text
    }

    fn render_def(&self, def: &Def, indent: &str, text: &mut String) {
        let shape = &def.shape;
        let fault = self.shown_fault(def);
        let result = match fault {
            Some(Fault::Signature) => format!(": Missing{}", def.id),
            _ if shape.declared || def.kind == Kind::Inline => format!(": {}", shape.result.name()),
            _ => String::new(),
        };
        let params: Vec<String> = shape
            .params
            .iter()
            .enumerate()
            .map(|(i, param)| {
                let default = match &param.default {
                    Some(value) => format!(" = {}", self.render_as(value, param.ty, def)),
                    None => String::new(),
                };
                format!("x{i}: {}{default}", param.ty.name())
            })
            .collect();
        let head = match def.kind {
            Kind::Main => "def main(args: Array[String]): Unit".to_string(),
            Kind::Def => format!("def d{}({}){result}", def.id, params.join(", ")),
            Kind::Inline => format!("inline def d{}({}){result}", def.id, params.join(", ")),
            Kind::Val => format!("val d{}{result}", def.id),
        };
        text.push_str(&format!("{indent}{head} =\n"));
        let mut declared = Vec::new();
        for statement in &shape.statements {
            match statement {
                Stmt::Val(n, value) => {
                    let value = self.render_expr(value, def, &declared);
                    text.push_str(&format!("{indent}  val t{n} = {value}\n"));
                    declared.push((*n, self.ty_in(statement_value(statement), def, &declared)));
                }
                Stmt::Print(value) => {
                    let value = self.render_expr(value, def, &declared);
                    text.push_str(&format!("{indent}  println({value})\n"));
                }
            }
        }
        let value = match (def.kind, fault) {
            (Kind::Main, _) => "()".to_string(),
            (_, Some(Fault::Unknown)) => format!("unknown{}", def.id),
            (_, Some(Fault::Mismatch)) => match shape.result {
                Ty::Int => "\"mismatch\"".to_string(),
                Ty::Str => "1".to_string(),
            },
            _ if shape.declared || def.kind == Kind::Inline => self.render_typed(&shape.value, shape.result, def, &declared),
            _ => self.render_expr(&shape.value, def, &declared),
        };
        text.push_str(&format!("{indent}  {value}\n"));
    }

    /// The type a definition has for its callers.
    pub fn result_of(&self, def: &Def) -> Ty {
        if def.shape.declared || def.kind == Kind::Inline {
            def.shape.result
        } else {
            self.ty_in(&def.shape.value, def, &self.locals_of(def))
        }
    }

    fn locals_of(&self, def: &Def) -> Vec<(usize, Ty)> {
        let mut declared = Vec::new();
        for statement in &def.shape.statements {
            if let Stmt::Val(n, value) = statement {
                let ty = self.ty_in(value, def, &declared);
                declared.push((*n, ty));
            }
        }
        declared
    }

    fn ty_in(&self, e: &Expr, def: &Def, locals: &[(usize, Ty)]) -> Ty {
        match e {
            Expr::Int(_) | Expr::Seed => Ty::Int,
            Expr::Str(_) => Ty::Str,
            Expr::Param(i) => def.shape.params.get(*i).map_or(Ty::Int, |param| param.ty),
            Expr::Local(n) => locals.iter().find(|(m, _)| m == n).map_or(Ty::Int, |(_, ty)| *ty),
            Expr::Call { def: callee, .. } => match self.callee(*callee, def) {
                Some(callee) => self.result_of(callee),
                None => Ty::Int,
            },
            Expr::Macro(Macro::Shape, _) => Ty::Str,
            Expr::Macro(..) => Ty::Int,
            Expr::Add(ty, ..) => *ty,
        }
    }

    /// A definition may call the ones made before it, so that no call graph has a cycle; the
    /// entry point calls any.
    fn callee(&self, callee: usize, caller: &Def) -> Option<&Def> {
        let found = self.def(callee)?;
        (found.kind != Kind::Main && (caller.kind == Kind::Main || callee < caller.id)).then_some(found)
    }

    fn render_as(&self, e: &Expr, ty: Ty, def: &Def) -> String {
        self.render_typed(e, ty, def, &[])
    }

    fn render_typed(&self, e: &Expr, ty: Ty, def: &Def, locals: &[(usize, Ty)]) -> String {
        let text = self.render_expr(e, def, locals);
        match (self.ty_in(e, def, locals), ty) {
            (Ty::Int, Ty::Str) => format!("({text}).toString"),
            (Ty::Str, Ty::Int) => format!("({text}).length"),
            _ => text,
        }
    }

    fn render_expr(&self, e: &Expr, def: &Def, locals: &[(usize, Ty)]) -> String {
        match e {
            Expr::Int(n) if *n < 0 => format!("({n})"),
            Expr::Int(n) => n.to_string(),
            Expr::Str(text) => quoted(text),
            Expr::Param(i) if *i < def.shape.params.len() && def.kind != Kind::Main => format!("x{i}"),
            Expr::Local(n) if locals.iter().any(|(m, _)| m == n) => format!("t{n}"),
            Expr::Seed if def.owner == Owner::Class => "seed".to_string(),
            Expr::Param(_) | Expr::Local(_) | Expr::Seed => "0".to_string(),
            Expr::Macro(Macro::Count, text) => format!("count({})", quoted(text)),
            Expr::Macro(Macro::Shape, text) => format!("shape({})", quoted(text)),
            Expr::Macro(Macro::Words, text) => format!("words{}", quoted(text)),
            Expr::Add(ty, a, b) => {
                format!("({} + {})", self.render_typed(a, *ty, def, locals), self.render_typed(b, *ty, def, locals))
            }
            Expr::Call { def: callee, seed, args } => {
                let Some(callee) = self.callee(*callee, def) else {
                    return "0".to_string();
                };
                let file = self.file(callee.file);
                let beside = callee.file == def.file && callee.owner == def.owner;
                let path = match callee.owner {
                    Owner::Object | Owner::Class if beside => format!("d{}", callee.id),
                    Owner::Top => format!("{}.d{}", file.package_name(), callee.id),
                    Owner::Object => format!("{}.O{}.d{}", file.package_name(), file.id, callee.id),
                    Owner::Class => {
                        format!("new {}.C{}({}).d{}", file.package_name(), file.id, Expr::Int(*seed).plain(), callee.id)
                    }
                };
                if callee.kind == Kind::Val {
                    return path;
                }
                let params = &callee.shape.params;
                let mut passed = Vec::new();
                for (i, param) in params.iter().enumerate() {
                    match args.get(i) {
                        Some(arg) => passed.push(self.render_typed(arg, param.ty, def, locals)),
                        None if params[i..].iter().all(|later| later.default.is_some()) => break,
                        None => passed.push(match param.ty {
                            Ty::Int => "0".to_string(),
                            Ty::Str => "\"\"".to_string(),
                        }),
                    }
                }
                format!("{path}({})", passed.join(", "))
            }
        }
    }
}

impl Expr {
    fn plain(&self) -> String {
        match self {
            Expr::Int(n) if *n < 0 => format!("({n})"),
            Expr::Int(n) => n.to_string(),
            _ => unreachable!(),
        }
    }

    /// The literals of the expression in the order of the text, for the rule that changes one.
    pub fn literals<'a>(&'a mut self, found: &mut Vec<&'a mut Expr>) {
        match self {
            Expr::Int(_) | Expr::Str(_) | Expr::Macro(..) => found.push(self),
            Expr::Call { args, .. } => args.iter_mut().for_each(|arg| arg.literals(found)),
            Expr::Add(_, a, b) => {
                a.literals(found);
                b.literals(found);
            }
            Expr::Param(_) | Expr::Local(_) | Expr::Seed => {}
        }
    }
}

impl Shape {
    pub fn literals(&mut self) -> Vec<&mut Expr> {
        let mut found = Vec::new();
        for statement in &mut self.statements {
            match statement {
                Stmt::Val(_, value) | Stmt::Print(value) => value.literals(&mut found),
            }
        }
        self.value.literals(&mut found);
        found
    }
}

fn statement_value(statement: &Stmt) -> &Expr {
    match statement {
        Stmt::Val(_, value) | Stmt::Print(value) => value,
    }
}

fn quoted(text: &str) -> String {
    let mut out = String::from("\"");
    for c in text.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\t' => out.push_str("\\t"),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}
