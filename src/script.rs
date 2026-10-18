//! Scala CLI scripts (`.sc`): a script's statements are the body of a class that Scala CLI 1.17.1
//! wraps them in for Scala 3 on the JVM (`ClassCodeWrapper`, `ScriptPreprocessor`), so that `teq interp`,
//! `teq compiler check` and the language server read a script as Scala CLI compiles it.
//!
//! For `hello-world.sc` the wrapper is the class `hello$minusworld$_` holding the script's code with
//! its `args` and `scriptPath`, the object `hello$minusworld_sc` that keeps the arguments it was run with
//! and makes the class's one instance lazily, its `main`, and the top-level
//! `export hello$minusworld_sc.script as hello-world` through which another script reaches this one's
//! definitions (``` `hello-world`.x ```). The class, the object and a directory's package components are
//! named as `ClassCodeWrapper` names them, encoded (`Name.backticked`, which encodes before it quotes;
//! `AmmUtil.encodeScalaSourcePath`), so that the source refers to them as Scala CLI's generated file
//! declares them (`hello$minusworld_sc.args$opt`, `x$minusy.helper`); the export alone keeps the raw name.
//! Only the script run gets arguments: another one reading its own throws Scala CLI's "No arguments
//! passed to this script".
//!
//! The wrapper is text appended after the script's own (`wrap`), so that every position in the script
//! stays its own (its lines, its columns, the diagnostics at them); the parser starts at the wrapper,
//! and where the class's body names `` `<script>` `` it reads the script's statements, from the file's
//! start to the wrapper (`parser::Syntax::script`).

/// The line that opens the wrapper appended to a script's text (`suffix_at`).
const MARKER: &str = "\n/*<teq: the Scala CLI script wrapper>*/\n";

/// The placeholder statement of the wrapper class's body that stands for the script's statements.
pub const BODY: &str = "<script>";

/// Whether the file at `path` is a Scala CLI script.
pub fn is_script(path: &str) -> bool {
    path.ends_with(".sc")
}

/// The name of the script at `path`: its file name without `.sc` (Scala CLI's `pathToPackageWrapper`).
pub fn name(path: &str) -> &str {
    let file = path.rsplit(['/', '\\']).next().unwrap_or(path);
    file.strip_suffix(".sc").unwrap_or(file)
}

/// The name of the object whose `main` runs the script, `<name>_sc` encoded (`x-y.sc`'s `x$minusy_sc`):
/// its name in the wrapper's source, as `ClassCodeWrapper` writes it (`Name.backticked`), and its class's,
/// which the entry point's choice reads as `--main` names it, as Scala CLI's `--main-class` does.
pub fn main_object(path: &str) -> String {
    crate::jvm::names::encode(&format!("{}_sc", name(path)))
}

/// Where the wrapper of a script's text starts, if the text is a wrapped script's.
pub fn suffix_at(path: &str, text: &str) -> Option<u32> {
    if !is_script(path) {
        return None;
    }
    text.rfind(MARKER).map(|at| at as u32)
}

/// How the object's `main` runs the script: the `main` of the script's one object with one
/// (`object X { def main(args: Array[String]) ... }` or `extends App`) when the script has no other
/// statement, else the script's own statements (Scala CLI's `WrapperUtils.mainObjectInScript`).
pub enum MainInvocation {
    Statements,
    Object(String),
}

/// How the script's object runs it: Scala CLI's `WrapperUtils.mainObjectInScript` over the script's
/// parsed top-level statements, which the parser reads as it reads them in the wrapper (`<script>`
/// in a class's body). An object is a candidate when its template's parents are `App` alone
/// (`extendsApp`) or one of its own members is a `main` of `checkSignature`'s shape (`main_signature`);
/// a term is a top-level statement (`noToplevelStatements`); a script that does not parse has none
/// (`Parsed.Success` alone gives statements).
pub fn main_invocation(text: &str) -> MainInvocation {
    use crate::ast::{ClassKind, DefKind, Stmt, TyExpr};
    let interner = crate::intern::Interner::new();
    let mut probe = String::with_capacity(text.len() + MARKER.len() + 32);
    probe.push_str(text);
    if !text.ends_with('\n') {
        probe.push('\n');
    }
    let at = probe.len() as u32;
    probe.push_str(MARKER);
    probe.push_str(&format!("final class S {{\n`{}`\n}}\n", BODY));
    let syntax = crate::parser::Syntax { script: Some(at), ..Default::default() };
    let (asts, found) = crate::frontend::parse_one(&probe, &interner, syntax);
    let Some(ast) = asts.first().filter(|_| found.errors.is_empty()) else { return MainInvocation::Statements };
    let wrapper = ast.top_level.iter().map(|&d| ast.def(d)).find_map(|d| match &d.kind {
        DefKind::Class(c) if interner.get(d.name) == "S" => Some(c),
        _ => None,
    });
    let Some(wrapper) = wrapper else { return MainInvocation::Statements };
    let named = |t, n: &str| matches!(ast.ty(t), TyExpr::Name(x) if interner.get(x) == n);
    let mut objects = Vec::new();
    let mut terms = false;
    for stmt in &wrapper.body {
        match *stmt {
            Stmt::Expr(_) => terms = true,
            Stmt::Def(d) => {
                let def = ast.def(d);
                let DefKind::Class(c) = &def.kind else { continue };
                if c.kind != ClassKind::Object {
                    continue;
                }
                // `extendsApp`: `Init(Type.Name("App"), _, Nil) :: Nil`, the one parent with no argument clause.
                let extends_app = matches!(c.parents.as_slice(), [p] if p.args.is_empty() && named(p.ty, "App"));
                let has_main = c.body.iter().any(|m| match *m {
                    Stmt::Def(m) => interner.get(ast.def(m).name) == "main" && main_signature(ast, &interner, ast.def(m)),
                    _ => false,
                });
                if extends_app || has_main {
                    objects.push(interner.get(def.name).to_string());
                }
            }
            Stmt::Import(_) => {}
        }
    }
    match objects.as_slice() {
        [one] if !terms => MainInvocation::Object(one.clone()),
        _ => MainInvocation::Statements,
    }
}

/// Whether `def` is `WrapperUtils.checkSignature`'s `main`: no type parameter and one clause, neither
/// `using` nor `implicit`, of one parameter of any name, without modifier, annotation or default, of type
/// `Array[String]` (`Type.Apply(Type.Name("Array"), List(Type.Name("String")))`, however parenthesised).
fn main_signature(ast: &crate::ast::Ast, interner: &crate::intern::Interner, def: &crate::ast::Def) -> bool {
    use crate::ast::{DefKind, TyExpr};
    let DefKind::Fun(f) = &def.kind else { return false };
    let [clause] = f.clauses.as_slice() else { return false };
    let [param] = clause.params.as_slice() else { return false };
    let named = |t, n: &str| matches!(ast.ty(t), TyExpr::Name(x) if interner.get(x) == n);
    f.tparams.is_empty()
        && !f.is_extension
        && !clause.is_using
        && !clause.is_implicit
        && param.mods == 0
        && param.annots.len == 0
        && param.default.is_none()
        && matches!(ast.ty(param.ty), TyExpr::Apply(base, args) if named(base, "Array") && matches!(ast.ty_list(args), [arg] if named(*arg, "String")))
}

/// A program file's text as the compiler reads it: a script's with its wrapper, any other's as it is.
pub fn loaded(path: &str, text: String) -> String {
    loaded_in(path, text, &[])
}

/// `loaded` with the script's package: the directories between it and the input directory that held
/// it (Scala CLI's input `subPath`, `AmmUtil.pathToPackageWrapper`), none for a script named alone.
pub fn loaded_in(path: &str, text: String, package: &[String]) -> String {
    loaded_as(path, text, package, path)
}

/// `loaded_in` with the script's `scriptPath` (`main.rs`'s `script_path`).
pub fn loaded_as(path: &str, text: String, package: &[String], script_path: &str) -> String {
    if is_script(path) {
        wrap(path, &text, &main_invocation(&text), package, script_path)
    } else {
        text
    }
}

/// The text of the script at `path` with the wrapper appended: its package clause first, its arguments
/// read from the object, `scriptPath` `script_path`: as Scala CLI's `ScriptPreprocessor` gives it,
/// `inputArgPath.getOrElse(subPath.toString)`, the argument a script named on the command line was given
/// by, else its path below the input that held it.
pub fn wrap(path: &str, text: &str, main: &MainInvocation, package: &[String], script_path: &str) -> String {
    let raw = name(path);
    let class = crate::jvm::names::encode(&format!("{}$_", raw));
    let object = main_object(path);
    let invocation = match main {
        MainInvocation::Statements => "val _ = script.hashCode()".to_string(),
        MainInvocation::Object(o) => format!("script.`{}`.main(args)", o),
    };
    let script_path = script_path.replace('\\', "/");
    let mut out = String::with_capacity(text.len() + 1024);
    out.push_str(text);
    if !text.ends_with('\n') {
        out.push('\n');
    }
    out.push_str(MARKER);
    if !package.is_empty() {
        out.push_str(&format!("package {}\n", package.iter().map(|p| format!("`{}`", crate::jvm::names::encode(p))).collect::<Vec<_>>().join(".")));
    }
    out.push_str(&format!(
        "final class `{class}` {{\n\
         def args = `{object}`.`args$`\n\
         def scriptPath = \"\"\"{script_path}\"\"\"\n\
         `{BODY}`\n\
         }}\n\
         object `{object}` {{\n\
         private var `args$opt0` = Option.empty[Array[String]]\n\
         def `args$set`(args: Array[String]): Unit = {{\n\
         `args$opt0` = Some(args)\n\
         }}\n\
         def `args$opt`: Option[Array[String]] = `args$opt0`\n\
         def `args$`: Array[String] = `args$opt`.getOrElse {{\n\
         sys.error(\"No arguments passed to this script\")\n\
         }}\n\
         lazy val script = new `{class}`\n\
         def main(args: Array[String]): Unit = {{\n\
         `args$set`(args)\n\
         {invocation}\n\
         }}\n\
         }}\n\
         export `{object}`.script as `{raw}`\n"
    ));
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_script_keeps_its_text_and_gains_the_wrapper_after_it() {
        let text = "println(args.length)\n";
        let wrapped = wrap("dir/hello-world.sc", text, &MainInvocation::Statements, &[], "dir/hello-world.sc");
        assert!(wrapped.starts_with(text));
        assert_eq!(suffix_at("dir/hello-world.sc", &wrapped), Some(text.len() as u32));
        assert!(wrapped.contains("final class `hello$minusworld$_` {"));
        assert!(wrapped.contains("object `hello$minusworld_sc` {"));
        assert!(wrapped.contains("export `hello$minusworld_sc`.script as `hello-world`"));
        assert_eq!(main_object("dir/hello-world.sc"), "hello$minusworld_sc");
        assert_eq!(suffix_at("dir/hello.scala", &wrapped), None);
    }

    #[test]
    fn the_one_object_with_a_main_runs_when_no_statement_stands_beside_it() {
        let only = "// a comment\nobject Hello:\n  def main(args: Array[String]): Unit = println(1)\n";
        assert!(matches!(main_invocation(only), MainInvocation::Object(o) if o == "Hello"));
        let app = "object Hi extends App {\n  println(1)\n}\nval x = 1\n";
        assert!(matches!(main_invocation(app), MainInvocation::Object(o) if o == "Hi"));
        let beside = "object Hello:\n  def main(args: Array[String]): Unit = ()\nprintln(2)\n";
        assert!(matches!(main_invocation(beside), MainInvocation::Statements));
        assert!(matches!(main_invocation("println(args.length)\n"), MainInvocation::Statements));
        // A nested object's main is no member of the script's object, nor is another shape of main.
        let nested = "object Outer {\n  object Inner {\n    def main(args: Array[String]): Unit = ()\n  }\n}\n";
        assert!(matches!(main_invocation(nested), MainInvocation::Statements));
        let curried = "object A:\n  def main(args: Array[String])(using x: Int): Unit = ()\n";
        assert!(matches!(main_invocation(curried), MainInvocation::Statements));
        let generic = "object A:\n  def main[T](args: Array[String]): Unit = ()\n";
        assert!(matches!(main_invocation(generic), MainInvocation::Statements));
        let nested_app = "trait App\nobject Outer {\n  object Inner extends App {\n  }\n}\n";
        assert!(matches!(main_invocation(nested_app), MainInvocation::Statements));
        let app_with = "object A extends App with Serializable {\n  println(1)\n}\n";
        assert!(matches!(main_invocation(app_with), MainInvocation::Statements));
        let app_colon = "object A extends App:\n  println(1)\n";
        assert!(matches!(main_invocation(app_colon), MainInvocation::Object(o) if o == "A"));
        let parenthesised = "object Main {\n  def main(args: (Array[String])): Unit = println(\"ran\")\n}\n";
        assert!(matches!(main_invocation(parenthesised), MainInvocation::Object(o) if o == "Main"));
        let backticked = "object Main {\n  def main(`using`: Array[String]): Unit = println(\"ran\")\n}\n";
        assert!(matches!(main_invocation(backticked), MainInvocation::Object(o) if o == "Main"));
        let defaulted = "object Main {\n  def main(args: Array[String] = Array()): Unit = ()\n}\n";
        assert!(matches!(main_invocation(defaulted), MainInvocation::Statements));
        let using = "object Main {\n  def main(using args: Array[String]): Unit = ()\n}\n";
        assert!(matches!(main_invocation(using), MainInvocation::Statements));
        let qualified = "object Main {\n  def main(args: scala.Array[String]): Unit = ()\n}\n";
        assert!(matches!(main_invocation(qualified), MainInvocation::Statements));
        let unparsed = "object Main {\n  def main(args: Array[String]): Unit = (\n}\n";
        assert!(matches!(main_invocation(unparsed), MainInvocation::Statements));
        let braces = "object A extends B(1) {\n  def main(a: Array[String]): Unit = ()\n}\n";
        assert!(matches!(main_invocation(braces), MainInvocation::Object(o) if o == "A"));
    }

    #[test]
    fn a_script_in_a_directory_input_takes_its_package() {
        let wrapped = wrap("dirs/a/helper.sc", "val answer = 42\n", &MainInvocation::Statements, &["a".to_string()], "a/helper.sc");
        assert!(wrapped.contains("package `a`\nfinal class `helper$_`"));
    }
}
