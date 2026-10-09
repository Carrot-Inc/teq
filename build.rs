//! The std index: for every file under `std/`, the package-qualified names it defines at the top
//! level, the packages it has members in, and the packages where it holds givens that have no
//! name to ask for. The compiler enters a std file the first time a lookup asks for one of its
//! names (`src/typer/stdlib.rs`), so the index is what decides which std files a program pays
//! for. The scan is line-based and errs towards listing more: a name listed by mistake costs one
//! needless file entry, a name missed would leave a lookup unanswered.

use std::fmt::Write as _;
use std::path::{Path, PathBuf};

fn main() {
    println!("cargo:rerun-if-changed=std");
    println!("cargo:rerun-if-changed=build.rs");
    // The release of sbt-teq this compiler selects, its own version line (docs/TARGETS.md, "Releases"): the
    // plugin the language server adds to a build it exports (src/lsp/export.rs).
    println!("cargo:rerun-if-changed=integrations/sbt/plugin-version.txt");
    let plugin = std::fs::read_to_string("integrations/sbt/plugin-version.txt").expect("integrations/sbt/plugin-version.txt names the plugin's version");
    let plugin = plugin.trim();
    let numbers: Vec<&str> = plugin.split('.').collect();
    assert!(numbers.len() == 3 && numbers.iter().all(|n| !n.is_empty() && n.bytes().all(|b| b.is_ascii_digit())), "integrations/sbt/plugin-version.txt holds {plugin:?}, not <major>.<minor>.<patch>");
    println!("cargo:rustc-env=TEQ_PLUGIN_VERSION={plugin}");
    emit_git_hash();
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("std");
    let mut files = Vec::new();
    collect(&root, &mut files);
    files.sort();
    // The identity of the std's documents in the language server (src/typer/loader/attach.rs):
    // a hash of every file's path and text, so that another std's text is another document.
    let mut identity: u64 = 0xcbf2_9ce4_8422_2325;
    let scanned: Vec<(String, Scanned)> = files
        .iter()
        .map(|file| {
            let text = std::fs::read_to_string(file).expect("std file readable");
            let rel = file.strip_prefix(&root).unwrap().to_string_lossy().replace('\\', "/");
            for &b in rel.as_bytes().iter().chain(&[0]).chain(text.as_bytes()).chain(&[0]) {
                identity = (identity ^ b as u64).wrapping_mul(0x0100_0000_01b3);
            }
            (rel, scan(&text))
        })
        .collect();
    let all_defines: Vec<&(String, String, u8)> = scanned.iter().flat_map(|(_, s)| s.defines.iter()).collect();
    let mut out = String::new();
    out.push_str("pub(crate) static STD_INDEX: &[StdFileIndex] = &[\n");
    for (rel, scanned) in &scanned {
        let _ = writeln!(out, "    StdFileIndex {{");
        let _ = writeln!(out, "        path: \"<std>/{}\",", rel);
        let _ = writeln!(out, "        blocks: {},", scanned.blocks);
        let _ = writeln!(out, "        packages: &[{}],", quoted(&scanned.packages));
        let _ = write!(out, "        defines: &[");
        for (pkg, name, kind) in &scanned.defines {
            let _ = write!(out, "({:?}, {:?}, {}), ", pkg, name, kind);
        }
        let _ = writeln!(out, "],");
        let _ = write!(out, "        extends: &[");
        let mut parents: Vec<(String, String)> = Vec::new();
        for (pkg, head) in &scanned.extends {
            let parent = resolve_parent(pkg, head, &scanned.imports, &all_defines);
            if !parents.contains(&parent) {
                parents.push(parent);
            }
        }
        for (pkg, name) in &parents {
            let _ = write!(out, "({:?}, {:?}), ", pkg, name);
        }
        let _ = writeln!(out, "],");
        let _ = writeln!(out, "        unnamed_givens: &[{}],", quoted(&scanned.unnamed_givens));
        let _ = writeln!(out, "        predef: &[{}],", quoted(&scanned.predef));
        let _ = writeln!(out, "    }},");
    }
    out.push_str("];\n");
    let _ = writeln!(out, "pub(crate) static STD_IDENTITY: &str = \"{:016x}\";", identity);
    let dest = PathBuf::from(std::env::var("OUT_DIR").unwrap()).join("std_index.rs");
    std::fs::write(dest, out).expect("index written");
}

/// `TEQ_GIT_HASH`, the short hash of the checkout's HEAD, empty outside a git checkout.
fn emit_git_hash() {
    let git = |args: &[&str]| {
        std::process::Command::new("git")
            .args(args)
            .current_dir(env!("CARGO_MANIFEST_DIR"))
            .output()
            .ok()
            .filter(|out| out.status.success())
            .map(|out| String::from_utf8_lossy(&out.stdout).trim().to_string())
            .filter(|s| !s.is_empty())
    };
    // A tree without its repository (a mirror on a build machine) names its revision itself.
    println!("cargo:rerun-if-env-changed=TEQ_REVISION");
    let hash = git(&["rev-parse", "--short", "HEAD"]).or_else(|| std::env::var("TEQ_REVISION").ok()).unwrap_or_default();
    let flags = std::env::var("CARGO_ENCODED_RUSTFLAGS").unwrap_or_default();
    let build = if flags.split('\x1f').any(|f| f.starts_with("-Cprofile-use")) {
        " pgo"
    } else if flags.split('\x1f').any(|f| f.starts_with("-Cprofile-generate")) {
        " instrumented"
    } else {
        ""
    };
    println!("cargo:rustc-env=TEQ_GIT_HASH={}", format!("{}{}", hash, build).trim_start());
    let branch = git(&["symbolic-ref", "-q", "HEAD"]);
    for name in ["HEAD"].into_iter().map(String::from).chain(branch).chain(Some("packed-refs".to_string())) {
        if let Some(path) = git(&["rev-parse", "--path-format=absolute", "--git-path", &name]) {
            if Path::new(&path).exists() {
                println!("cargo:rerun-if-changed={}", path);
            }
        }
    }
}

fn quoted(items: &[String]) -> String {
    items.iter().map(|s| format!("{:?}", s)).collect::<Vec<_>>().join(", ")
}

fn collect(dir: &Path, out: &mut Vec<PathBuf>) {
    for entry in std::fs::read_dir(dir).expect("std directory readable") {
        let path = entry.expect("directory entry").path();
        if path.is_dir() {
            collect(&path, out);
        } else if path.extension().map_or(false, |e| e == "scala") {
            out.push(path);
        }
    }
}

/// What a definition can be asked for as: a type, a term, or either.
const TYPE: u8 = 1;
const TERM: u8 = 2;

#[derive(Default)]
struct Scanned {
    blocks: u32,
    packages: Vec<String>,
    defines: Vec<(String, String, u8)>,
    unnamed_givens: Vec<String>,
    /// The parents a class, object or enum of the file extends, as (the package of the
    /// definition, the head of the parent type as written).
    extends: Vec<(String, String)>,
    /// The import clauses of the file, as (path, names imported or none for a wildcard).
    imports: Vec<(String, Option<Vec<String>>)>,
    /// The names of the definitions marked `@predef`, and whether the next one is.
    predef: Vec<String>,
    predef_pending: bool,
}

impl Scanned {
    fn define(&mut self, pkg: &str, name: &str, kind: u8) {
        if std::mem::take(&mut self.predef_pending) && !self.predef.iter().any(|n| n == name) {
            self.predef.push(name.to_string());
        }
        if let Some(entry) = self.defines.iter_mut().find(|(p, n, _)| p == pkg && n == name) {
            entry.2 |= kind;
            return;
        }
        self.defines.push((pkg.to_string(), name.to_string(), kind));
    }

    fn package(&mut self, pkg: &str) {
        if !self.packages.iter().any(|p| p == pkg) {
            self.packages.push(pkg.to_string());
        }
    }

    fn unnamed_given(&mut self, pkg: &str) {
        if !self.unnamed_givens.iter().any(|p| p == pkg) {
            self.unnamed_givens.push(pkg.to_string());
        }
    }
}

/// A package block: the indentation of its `package p:` line and its full path. The header
/// clauses of the file are the block at indentation `usize::MAX`-less-than-everything, -1.
struct Block {
    indent: i64,
    path: String,
}

fn scan(text: &str) -> Scanned {
    let mut out = Scanned::default();
    let mut blocks: Vec<Block> = vec![Block { indent: -1, path: String::new() }];
    let mut in_header = true;
    let mut in_comment = false;
    // The indentation of an `extension` clause whose methods follow on their own lines.
    let mut extension_at: Option<i64> = None;
    for raw in text.lines() {
        let line = strip_comments(raw, &mut in_comment);
        let trimmed = line.trim_start();
        if trimmed.is_empty() {
            continue;
        }
        let indent = (line.len() - trimmed.len()) as i64;
        let words = words_of(trimmed);
        if words == ["@", "predef"] {
            out.predef_pending = true;
            continue;
        }
        if words.first().map(String::as_str) == Some("package") && words.get(1).map(String::as_str) != Some("object") {
            let Some(path) = words.get(1) else { continue };
            let path = path.as_str();
            let is_block = words.get(2).map(String::as_str) == Some(":") || words.get(2).map(String::as_str) == Some("{");
            if !is_block && in_header {
                let header = &mut blocks[0].path;
                if !header.is_empty() {
                    header.push('.');
                }
                header.push_str(path);
                out.package(&blocks[0].path.clone());
                continue;
            }
            in_header = false;
            while blocks.len() > 1 && blocks.last().unwrap().indent >= indent {
                blocks.pop();
            }
            let enclosing = &blocks.last().unwrap().path;
            let full = if enclosing.is_empty() { path.to_string() } else { format!("{}.{}", enclosing, path) };
            out.package(&full);
            out.blocks += 1;
            blocks.push(Block { indent, path: full });
            extension_at = None;
            continue;
        }
        if words.first().map(String::as_str) == Some("package") {
            // `package object p`: its members are the top-level definitions of `p`.
            in_header = false;
            while blocks.len() > 1 && blocks.last().unwrap().indent >= indent {
                blocks.pop();
            }
            let Some(name) = words.get(2) else { continue };
            let enclosing = &blocks.last().unwrap().path;
            let full = if enclosing.is_empty() { name.to_string() } else { format!("{}.{}", enclosing, name) };
            out.package(&full);
            out.blocks += 1;
            blocks.push(Block { indent, path: full });
            extension_at = None;
            continue;
        }
        while blocks.len() > 1 && blocks.last().unwrap().indent >= indent {
            blocks.pop();
        }
        let block = blocks.last().unwrap();
        let body_indent = if block.indent < 0 { 0 } else { block.indent + 2 };
        if indent <= body_indent {
            in_header = false;
        }
        if let Some(ext) = extension_at {
            if indent <= ext {
                extension_at = None;
            } else if indent == ext + 2 {
                if let Some(name) = def_name(&words, "def") {
                    out.define(&block.path.clone(), &name, TERM);
                }
                continue;
            } else {
                continue;
            }
        }
        if words.first().map(String::as_str) == Some("import") {
            out.imports.extend(import_clauses(&words[1..]));
            continue;
        }
        if indent != body_indent {
            continue;
        }
        let pkg = block.path.clone();
        if out.packages.is_empty() || !out.packages.iter().any(|p| *p == pkg) {
            out.package(&pkg);
        }
        if words.first().map(String::as_str) == Some("import") {
            out.imports.extend(import_clauses(&words[1..]));
            continue;
        }
        let Some((keyword, rest, implicit)) = definition_of(&words) else { continue };
        if implicit {
            out.unnamed_given(&pkg);
        }
        if matches!(keyword.as_str(), "class" | "object" | "enum") {
            for head in parent_heads(&rest) {
                out.extends.push((pkg.clone(), head));
            }
        }
        match keyword.as_str() {
            "class" | "trait" | "enum" => {
                if let Some(name) = rest.first() {
                    out.define(&pkg, &identifier(name), TYPE | TERM);
                }
            }
            "type" => {
                if let Some(name) = rest.first() {
                    out.define(&pkg, &identifier(name), TYPE);
                }
            }
            "object" | "def" | "val" | "var" => {
                if let Some(name) = rest.first() {
                    out.define(&pkg, &identifier(name), TERM);
                }
            }
            // A given is found by its type whether or not it has a name: the package's given
            // index enters the file.
            "given" => {
                out.unnamed_given(&pkg);
                if let Some(name) = given_name(&rest) {
                    out.define(&pkg, &name, TERM);
                }
            }
            "extension" => {
                // `extension (x: T) def m` on one line, or the methods on the lines below.
                if let Some(pos) = rest.iter().position(|w| w == "def") {
                    if let Some(name) = rest.get(pos + 1) {
                        out.define(&pkg, &identifier(name), TERM);
                    }
                } else {
                    extension_at = Some(indent);
                }
            }
            "export" => {
                for name in exported_names(&rest) {
                    out.define(&pkg, &name, TYPE | TERM);
                }
            }
            _ => {}
        }
    }
    out
}

/// The line without its `//` comment and the parts of `/* */` comments, tracking the latter
/// across lines. A `//` inside a string literal is kept.
fn strip_comments(line: &str, in_comment: &mut bool) -> String {
    let mut out = String::new();
    let bytes = line.as_bytes();
    let mut i = 0;
    let mut in_string = false;
    while i < bytes.len() {
        if *in_comment {
            if bytes[i..].starts_with(b"*/") {
                *in_comment = false;
                i += 2;
            } else {
                i += 1;
            }
            continue;
        }
        if in_string {
            if bytes[i] == b'\\' {
                i += 2;
                continue;
            }
            if bytes[i] == b'"' {
                in_string = false;
            }
            out.push(bytes[i] as char);
            i += 1;
            continue;
        }
        if bytes[i..].starts_with(b"//") {
            break;
        }
        if bytes[i..].starts_with(b"/*") {
            *in_comment = true;
            i += 2;
            continue;
        }
        if bytes[i] == b'"' {
            in_string = true;
        }
        out.push(bytes[i] as char);
        i += 1;
    }
    out
}

/// The words of a line: alphanumeric identifiers (with a trailing operator part after `_`),
/// operator identifiers, and brackets, parentheses, commas and braces as single-character
/// words, so that clauses can be told apart. A lone `:` or `=` is punctuation, `<:<` a name.
fn words_of(line: &str) -> Vec<String> {
    let is_op = |c: char| "!#%&*+-/:<=>?@\\^|~".contains(c);
    let mut out = Vec::new();
    let chars: Vec<char> = line.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        if c.is_whitespace() {
            i += 1;
        } else if matches!(c, '(' | ')' | '[' | ']' | ',' | '{' | '}' | '.' | ';') {
            out.push(c.to_string());
            i += 1;
        } else if c.is_alphanumeric() || c == '_' || c == '$' || c == '`' {
            let start = i;
            while i < chars.len() && (chars[i].is_alphanumeric() || chars[i] == '_' || chars[i] == '$' || chars[i] == '`' || chars[i] == '.') {
                i += 1;
            }
            // `unary_!`, `foo_=`
            if i < chars.len() && chars[i - 1] == '_' && is_op(chars[i]) {
                while i < chars.len() && is_op(chars[i]) {
                    i += 1;
                }
            }
            let word: String = chars[start..i].iter().collect();
            out.push(word.trim_end_matches('.').to_string());
        } else if is_op(c) {
            let start = i;
            while i < chars.len() && is_op(chars[i]) {
                i += 1;
            }
            let word: String = chars[start..i].iter().collect();
            if word == "=>" || word == ":" || word == "=" {
                out.push(word);
            } else if let Some(rest) = word.strip_suffix(':') {
                // `def ++:` is one name, `x:` a name and its colon; an operator name ends in
                // `:` only when written `name :`, which `given Int: Ordering[Int]` never is.
                out.push(rest.to_string());
                out.push(":".to_string());
            } else if let Some(rest) = word.strip_suffix('=').filter(|r| !r.is_empty() && !r.ends_with('=') && !r.ends_with('<') && !r.ends_with('>') && !r.ends_with('!')) {
                out.push(rest.to_string());
                out.push("=".to_string());
            } else {
                out.push(word);
            }
        } else {
            out.push(c.to_string());
            i += 1;
        }
    }
    out
}

const MODIFIERS: &[&str] = &[
    "private", "protected", "final", "abstract", "sealed", "open", "case", "implicit", "inline", "transparent", "lazy", "override", "opaque",
    "infix", "erased",
];

/// The definition keyword of a line, the words after it past the modifiers, and whether it is
/// `implicit`; none for a line that is no definition (an annotation, an import, a statement).
fn definition_of(words: &[String]) -> Option<(String, Vec<String>, bool)> {
    let mut i = 0;
    let mut implicit = false;
    while i < words.len() {
        let w = words[i].as_str();
        if MODIFIERS.contains(&w) {
            implicit |= w == "implicit";
            i += 1;
            // `private[pkg]`
            if words.get(i).map(String::as_str) == Some("[") {
                while i < words.len() && words[i] != "]" {
                    i += 1;
                }
                i += 1;
            }
            continue;
        }
        if w.starts_with('@') {
            return None;
        }
        return match w {
            "class" | "trait" | "object" | "enum" | "type" | "def" | "val" | "var" | "given" | "extension" | "export" => {
                Some((w.to_string(), words[i + 1..].to_vec(), implicit))
            }
            _ => None,
        };
    }
    None
}

/// The name of a `def` on a line of an extension block, past its modifiers and annotations.
fn def_name(words: &[String], keyword: &str) -> Option<String> {
    let (k, rest, _) = definition_of(words)?;
    (k == keyword).then(|| rest.first().map(|n| identifier(n))).flatten()
}

fn identifier(word: &str) -> String {
    word.to_string()
}

/// The name of a given, when it has one: `given name:`, `given name[T]:`, `given name(using ...):`
/// or the `with` forms; an anonymous given is `given Type[...] =` or `given Type[...] with`.
fn given_name(rest: &[String]) -> Option<String> {
    let name = rest.first()?;
    let mut i = 1;
    let mut depth = 0i32;
    while i < rest.len() {
        match rest[i].as_str() {
            "[" | "(" => depth += 1,
            "]" | ")" => depth -= 1,
            ":" if depth == 0 => return Some(identifier(name)),
            "=" | "with" if depth == 0 => return None,
            _ => {}
        }
        i += 1;
    }
    None
}

/// The heads of the parent types of a definition line: what follows `extends`, split at `with`
/// and commas, each up to its type arguments or constructor arguments.
fn parent_heads(rest: &[String]) -> Vec<String> {
    let Some(at) = rest.iter().position(|w| w == "extends") else { return Vec::new() };
    let mut out = Vec::new();
    let mut depth = 0i32;
    let mut expect_head = true;
    for w in &rest[at + 1..] {
        match w.as_str() {
            "[" | "(" | "{" => depth += 1,
            "]" | ")" | "}" => depth -= 1,
            "with" | "," if depth == 0 => expect_head = true,
            ":" | "=" | "=>" if depth == 0 => break,
            _ if depth == 0 && expect_head => {
                if w.chars().next().map_or(false, |c| c.is_alphabetic() || c == '_') {
                    out.push(w.clone());
                }
                expect_head = false;
            }
            _ => {}
        }
        if depth < 0 {
            break;
        }
    }
    out
}

/// An import line's clauses: `import a.b.C`, `import a.b.{C, D as E}`, `import a.b.*`, and
/// several separated by commas.
fn import_clauses(words: &[String]) -> Vec<(String, Option<Vec<String>>)> {
    let text = words.join(" ");
    let mut out = Vec::new();
    let mut rest = text.as_str();
    while !rest.trim().is_empty() {
        let rest_trim = rest.trim_start();
        let (clause, after) = match rest_trim.find('{') {
            Some(open) if rest_trim[..open].trim().ends_with('.') || rest_trim[..open].trim_end().ends_with(". ") || !rest_trim[..open].contains(',') => {
                let close = rest_trim.find('}').unwrap_or(rest_trim.len() - 1);
                (&rest_trim[..close + 1], &rest_trim[close + 1..])
            }
            _ => match rest_trim.find(',') {
                Some(comma) => (&rest_trim[..comma], &rest_trim[comma + 1..]),
                None => (rest_trim, ""),
            },
        };
        let clause = clause.trim().trim_end_matches(',').trim();
        rest = after.trim_start_matches(',');
        if clause.is_empty() {
            continue;
        }
        if let Some(open) = clause.find('{') {
            let path = clause[..open].trim().trim_end_matches('.').replace(' ', "");
            let inner = clause[open + 1..].trim_end_matches('}');
            let names: Vec<String> = inner
                .split(',')
                .filter_map(|sel| {
                    let sel = sel.trim();
                    if sel.is_empty() {
                        return None;
                    }
                    let name = match sel.split_once(" as ").or_else(|| sel.split_once("=>")) {
                        Some((_, alias)) => alias.trim(),
                        None => sel,
                    };
                    Some(name.to_string())
                })
                .collect();
            out.push((path, Some(names)));
        } else {
            let path = clause.replace(' ', "");
            match path.rsplit_once('.') {
                Some((prefix, "*")) | Some((prefix, "_")) => out.push((prefix.to_string(), None)),
                Some((prefix, name)) => out.push((prefix.to_string(), Some(vec![name.to_string()]))),
                None => {}
            }
        }
    }
    out
}

/// The package and name a parent type's head resolves to among the std's definitions: a
/// qualified head through the import that names its first segment or as an absolute path, a
/// simple one in the definition's package, its named imports, its wildcard imports, `scala`
/// and `java.lang`, in that order; an unresolved head keeps an empty package and is matched by
/// name alone.
fn resolve_parent(pkg: &str, head: &str, imports: &[(String, Option<Vec<String>>)], defines: &[&(String, String, u8)]) -> (String, String) {
    let defined = |p: &str, n: &str| defines.iter().any(|(dp, dn, k)| dp == p && dn == n && k & TYPE != 0);
    if let Some((first, rest)) = head.split_once('.') {
        let prefix = imports
            .iter()
            .find_map(|(path, names)| match names {
                Some(names) if names.iter().any(|n| n == first) => Some(format!("{}.{}", path, first)),
                None if defined(path, first) => Some(format!("{}.{}", path, first)),
                _ => None,
            })
            .unwrap_or_else(|| first.to_string());
        let full = format!("{}.{}", prefix, rest);
        let (p, n) = full.rsplit_once('.').unwrap();
        return (p.to_string(), n.to_string());
    }
    if defined(pkg, head) {
        return (pkg.to_string(), head.to_string());
    }
    for (path, names) in imports {
        match names {
            Some(names) if names.iter().any(|n| n == head) => return (path.clone(), head.to_string()),
            None if defined(path, head) => return (path.clone(), head.to_string()),
            _ => {}
        }
    }
    for p in ["scala", "java.lang"] {
        if defined(p, head) {
            return (p.to_string(), head.to_string());
        }
    }
    (String::new(), head.to_string())
}

/// The names an `export a.b.{C, D => E}` or `export a.b.C` clause brings into its package; a
/// wildcard is recorded as `*`.
fn exported_names(rest: &[String]) -> Vec<String> {
    let mut out = Vec::new();
    let text = rest.join(" ");
    if let Some(open) = text.find('{') {
        let inner = text[open + 1..].trim_end_matches('}').trim_end_matches(" }");
        let inner = inner.split('}').next().unwrap_or("");
        for sel in inner.split(',') {
            let sel = sel.trim();
            if sel.is_empty() {
                continue;
            }
            let name = match sel.split_once("=>") {
                Some((_, alias)) => alias.trim(),
                None => sel,
            };
            out.push(name.to_string());
        }
    } else {
        let path = text.split_whitespace().next().unwrap_or("");
        let last = path.rsplit('.').next().unwrap_or("");
        out.push(last.to_string());
    }
    out
}
