//! The configuration of `teq fix` (docs/FIX.md, "Configuration"): `.scalafix.conf` beside `teq.lock` or the file
//! `--config` names, read as scalafix reads it, through metaconfig 0.18.6 over typesafe-config 1.4.6, whose
//! `ConfigFactory.parseFile(file).resolve()` this ports. Each file is parsed into typesafe-config's values as
//! `ConfigParser` builds them (`Values`, an arena whose indices stand for the Java objects' identities), an `include`
//! read where it stands, and the whole resolved as `ResolveContext` resolves it (`Resolver`) into a `Conf` with every
//! value's place. What it reads: `key = value`, `key: value`, `key { ... }` and `key += value`, dotted keys, objects
//! merged as HOCON merges them, lists with or without commas, quoted and bare strings, values concatenated on a line,
//! numbers, booleans, `null`, `#` and `//` comments, `include` (of a quoted name, `file(...)`, `required(...)`) and the
//! substitutions `${path}` and `${?path}`, the environment standing for a key the configuration lacks. A triple-quoted
//! string, `url(...)`, `classpath(...)`, an included Java properties file and an included file that is not there (which
//! typesafe-config would look for on the JVM's class path) are errors at their place, never skipped.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::rc::Rc;

/// A value with where it was written.
#[derive(Clone, Debug)]
pub struct Node {
    pub value: Value,
    /// Its file, an index of `Conf::files`.
    pub file: u32,
    pub line: u32,
    pub col: u32,
}

#[derive(Clone, Debug)]
pub enum Value {
    Str(String),
    /// A number as written.
    Num(String),
    Bool(bool),
    Null,
    List(Vec<u32>),
    Obj(Vec<(String, u32)>),
}

/// A configuration: its nodes, the root object first, and the files they were read from, the main one first.
#[derive(Clone, Debug, Default)]
pub struct Conf {
    pub file: String,
    pub files: Vec<String>,
    pub nodes: Vec<Node>,
}

impl Conf {
    pub fn empty(file: &str) -> Conf {
        Conf { file: file.to_string(), files: vec![file.to_string()], nodes: vec![Node { value: Value::Obj(Vec::new()), file: 0, line: 1, col: 1 }] }
    }

    pub fn place(&self, n: u32) -> String {
        let node = &self.nodes[n as usize];
        format!("{}:{}:{}", self.files[node.file as usize], node.line, node.col)
    }

    /// The field `key` of the object `n`.
    pub fn field(&self, n: u32, key: &str) -> Option<u32> {
        match &self.nodes[n as usize].value {
            Value::Obj(fields) => fields.iter().find(|(k, _)| k == key).map(|&(_, v)| v),
            _ => None,
        }
    }

    /// The value at a dotted path from the root (`lint.error.includes`).
    pub fn at(&self, path: &[&str]) -> Option<u32> {
        let mut n = 0;
        for k in path {
            n = self.field(n, k)?;
        }
        Some(n)
    }

    pub fn str(&self, n: u32) -> Option<String> {
        match &self.nodes[n as usize].value {
            Value::Str(s) | Value::Num(s) => Some(s.clone()),
            Value::Bool(b) => Some(b.to_string()),
            _ => None,
        }
    }

    /// A string or a list of strings.
    pub fn strings(&self, n: u32) -> Result<Vec<String>, String> {
        match &self.nodes[n as usize].value {
            Value::List(items) => items.iter().map(|&i| self.str(i).ok_or_else(|| format!("{}: expected a string", self.place(i)))).collect(),
            Value::Null => Ok(Vec::new()),
            _ => self.str(n).map(|s| vec![s]).ok_or_else(|| format!("{}: expected a string or a list of strings", self.place(n))),
        }
    }
}

/// The configuration at `path`, its includes read beside it, its substitutions resolved.
pub fn load(path: &Path) -> Result<Conf, String> {
    load_with(path, &|p: &Path| std::fs::read_to_string(p), &|k: &str| std::env::var(k).ok())
}

/// `load` over the files `read` gives and the environment `env` gives.
pub fn load_with(path: &Path, read: &dyn Fn(&Path) -> std::io::Result<String>, env: &dyn Fn(&str) -> Option<String>) -> Result<Conf, String> {
    let text = read(path).map_err(|e| format!("cannot read {}: {}", path.display(), e))?;
    let mut ld = Loader { v: Values::default(), read, open: Vec::new() };
    let root = ld.parse_file(path, &text)?;
    let resolved = Resolver { v: &mut ld.v, env }.resolve_root(root)?;
    let mut conf = Conf { file: path.to_string_lossy().into_owned(), files: Vec::new(), nodes: Vec::new() };
    copy(&ld.v, resolved, &mut conf);
    conf.files = ld.v.files;
    Ok(conf)
}

/// The resolved value `id` and everything under it copied into `conf`, each occurrence a node of its own (a
/// substitution's value, written once, can stand in several places).
fn copy(v: &Values, id: Id, conf: &mut Conf) -> u32 {
    let at = v.nodes[id as usize].at;
    let n = conf.nodes.len() as u32;
    conf.nodes.push(Node { value: Value::Null, file: at.file, line: at.line, col: at.col });
    let value = match &v.nodes[id as usize].raw {
        Raw::Str(s, _) => Value::Str(s.clone()),
        Raw::Num(s) => Value::Num(s.clone()),
        Raw::Bool(b) => Value::Bool(*b),
        Raw::Null => Value::Null,
        Raw::List(items) => Value::List(items.clone().into_iter().map(|i| copy(v, i, conf)).collect()),
        Raw::Obj(fields, _) => Value::Obj(fields.clone().into_iter().map(|(k, f)| (k, copy(v, f, conf))).collect()),
        Raw::Ref(..) | Raw::Concat(_) | Raw::Merge(_) | Raw::MergeObj(_) => unreachable!("an unresolved value after resolution"),
    };
    conf.nodes[n as usize].value = value;
    n
}

type Id = u32;

/// Where a value was written: its file (an index of the files read), line and column.
#[derive(Clone, Copy, Debug)]
struct Place {
    file: u32,
    line: u32,
    col: u32,
}

/// A value as typesafe-config's tree holds it before resolution (`AbstractConfigValue`'s classes).
#[derive(Clone, Debug)]
enum Raw {
    /// A string, and whether it was quoted (`ConfigString.Quoted` or `Unquoted`).
    Str(String, bool),
    Num(String),
    Bool(bool),
    Null,
    List(Vec<Id>),
    /// An object's fields, in the order first written, and whether it ignores fallbacks (`SimpleConfigObject`).
    Obj(Vec<(String, Id)>, bool),
    /// `${path}`: the path, whether optional (`${?path}`), and the length of the prefix an include nested in an object
    /// gave it (`ConfigReference.prefixLength`).
    Ref(Vec<String>, bool, usize),
    /// Values written side by side, one of them unresolved (`ConfigConcatenation`).
    Concat(Vec<Id>),
    /// Values merged once resolved, the earlier winning (`ConfigDelayedMerge`), and one known to be an object
    /// (`ConfigDelayedMergeObject`).
    Merge(Vec<Id>),
    MergeObj(Vec<Id>),
}

struct RawNode {
    raw: Raw,
    at: Place,
    /// `ResolveStatus.fromValues`: no substitution in it.
    resolved: bool,
}

/// The values of every file read and every value resolution makes, never changed once made: an index is a value's
/// identity, as typesafe-config compares its objects. With the files' names, which places name.
#[derive(Default)]
struct Values {
    nodes: Vec<RawNode>,
    files: Vec<String>,
}

impl Values {
    fn make(&mut self, raw: Raw, at: Place) -> Id {
        let resolved = match &raw {
            Raw::Str(..) | Raw::Num(_) | Raw::Bool(_) | Raw::Null => true,
            Raw::List(items) => items.iter().all(|&i| self.resolved(i)),
            Raw::Obj(fields, _) => fields.iter().all(|&(_, f)| self.resolved(f)),
            Raw::Ref(..) | Raw::Concat(_) | Raw::Merge(_) | Raw::MergeObj(_) => false,
        };
        self.nodes.push(RawNode { raw, at, resolved });
        (self.nodes.len() - 1) as Id
    }

    fn raw(&self, id: Id) -> &Raw {
        &self.nodes[id as usize].raw
    }

    fn at(&self, id: Id) -> Place {
        self.nodes[id as usize].at
    }

    fn resolved(&self, id: Id) -> bool {
        self.nodes[id as usize].resolved
    }

    /// `ConfigObject`: a `SimpleConfigObject` or a `ConfigDelayedMergeObject`.
    fn is_object(&self, id: Id) -> bool {
        matches!(self.raw(id), Raw::Obj(..) | Raw::MergeObj(_))
    }

    fn is_unmergeable(&self, id: Id) -> bool {
        matches!(self.raw(id), Raw::Ref(..) | Raw::Concat(_) | Raw::Merge(_) | Raw::MergeObj(_))
    }

    /// `Container`: what `replaceChild` applies to.
    fn is_container(&self, id: Id) -> bool {
        matches!(self.raw(id), Raw::Obj(..) | Raw::List(_) | Raw::Concat(_) | Raw::Merge(_) | Raw::MergeObj(_))
    }

    fn ignores_fallbacks(&self, id: Id) -> bool {
        match self.raw(id) {
            Raw::Obj(_, ignores) => *ignores,
            Raw::Merge(stack) | Raw::MergeObj(stack) => self.ignores_fallbacks(*stack.last().unwrap()),
            Raw::Ref(..) | Raw::Concat(_) => false,
            _ => self.resolved(id),
        }
    }

    fn unmerged(&self, id: Id) -> Vec<Id> {
        match self.raw(id) {
            Raw::Merge(stack) | Raw::MergeObj(stack) => stack.clone(),
            _ => vec![id],
        }
    }

    fn field(&self, obj: Id, key: &str) -> Option<Id> {
        match self.raw(obj) {
            Raw::Obj(fields, _) => fields.iter().find(|(k, _)| k == key).map(|&(_, v)| v),
            _ => None,
        }
    }

    /// `AbstractConfigValue.withFallback`: `this` over `other`.
    fn with_fallback(&mut self, this: Id, other: Id) -> Id {
        if self.ignores_fallbacks(this) {
            return this;
        }
        if self.is_unmergeable(other) {
            // `mergedWithTheUnmergeable`.
            let mut stack = self.unmerged(this);
            stack.extend(self.unmerged(other));
            return self.delayed(this, stack);
        }
        if matches!(self.raw(this), Raw::Obj(..)) && matches!(self.raw(other), Raw::Obj(..)) {
            return self.merge_objects(this, other);
        }
        // `mergedWithNonObject` (a `ConfigDelayedMergeObject`'s `mergedWithObject` too): a resolved object falling back
        // to anything else no longer takes fallbacks; an unresolved value keeps the fallback for its resolution.
        if self.resolved(this) {
            match self.raw(this).clone() {
                Raw::Obj(fields, _) => self.make(Raw::Obj(fields, true), self.at(this)),
                _ => this,
            }
        } else {
            let mut stack = self.unmerged(this);
            stack.push(other);
            self.delayed(this, stack)
        }
    }

    /// `constructDelayedMerge`: an object's delayed merge is known to be one.
    fn delayed(&mut self, this: Id, stack: Vec<Id>) -> Id {
        let raw = if self.is_object(this) { Raw::MergeObj(stack) } else { Raw::Merge(stack) };
        self.make(raw, self.at(this))
    }

    /// `SimpleConfigObject.mergedWithObject`: per key, `this`'s over `fallback`'s; the merged object at the earlier one's
    /// place, its keys in the order first written.
    fn merge_objects(&mut self, this: Id, fallback: Id) -> Id {
        let (Raw::Obj(mine, _), Raw::Obj(theirs, _)) = (self.raw(this).clone(), self.raw(fallback).clone()) else { unreachable!() };
        let mut keys: Vec<String> = theirs.iter().map(|(k, _)| k.clone()).collect();
        keys.extend(mine.iter().map(|(k, _)| k.clone()).filter(|k| !theirs.iter().any(|(t, _)| t == k)));
        let mut changed = false;
        let mut merged = Vec::with_capacity(keys.len());
        for k in keys {
            let first = mine.iter().find(|(m, _)| *m == k).map(|&(_, v)| v);
            let second = theirs.iter().find(|(t, _)| *t == k).map(|&(_, v)| v);
            let kept = match (first, second) {
                (Some(a), Some(b)) => self.with_fallback(a, b),
                (Some(a), None) => a,
                (None, Some(b)) => b,
                (None, None) => unreachable!(),
            };
            changed |= first != Some(kept);
            merged.push((k, kept));
        }
        let ignores = self.ignores_fallbacks(fallback);
        if changed {
            self.make(Raw::Obj(merged, ignores), self.at(fallback))
        } else if ignores != self.ignores_fallbacks(this) {
            self.make(Raw::Obj(mine, ignores), self.at(this))
        } else {
            this
        }
    }

    /// `replaceChild`: `parent` with its child `old` replaced by `new`, or removed; none where a list or a stack is
    /// left empty.
    fn replace_child(&mut self, parent: Id, old: Id, new: Option<Id>) -> Option<Id> {
        let at = self.at(parent);
        let in_list = |items: &[Id]| -> Option<Vec<Id>> {
            let i = items.iter().position(|&x| x == old).expect("replaceChild of a value not in its parent");
            let mut out = items.to_vec();
            match new {
                Some(n) => out[i] = n,
                None => {
                    out.remove(i);
                }
            }
            (!out.is_empty()).then_some(out)
        };
        let raw = match self.raw(parent).clone() {
            Raw::Obj(fields, ignores) => {
                let i = fields.iter().position(|&(_, v)| v == old).expect("replaceChild of a value not in its object");
                let mut fields = fields;
                match new {
                    Some(n) => fields[i].1 = n,
                    None => {
                        fields.remove(i);
                    }
                }
                Raw::Obj(fields, ignores)
            }
            Raw::List(items) => Raw::List(in_list(&items)?),
            Raw::Concat(pieces) => Raw::Concat(in_list(&pieces)?),
            Raw::Merge(stack) => Raw::Merge(in_list(&stack)?),
            Raw::MergeObj(stack) => Raw::MergeObj(in_list(&stack)?),
            _ => unreachable!("replaceChild of a value that is no container"),
        };
        Some(self.make(raw, at))
    }

    /// `relativized`: every substitution under `id` prefixed with `prefix`, the path an include nested in an object
    /// stands at.
    fn relativized(&mut self, id: Id, prefix: &[String]) -> Id {
        let at = self.at(id);
        let raw = match self.raw(id).clone() {
            Raw::Ref(path, optional, len) => Raw::Ref(prefix.iter().cloned().chain(path).collect(), optional, len + prefix.len()),
            Raw::Obj(fields, ignores) => Raw::Obj(fields.into_iter().map(|(k, f)| (k, self.relativized(f, prefix))).collect(), ignores),
            Raw::List(items) => Raw::List(items.into_iter().map(|i| self.relativized(i, prefix)).collect()),
            Raw::Concat(pieces) => Raw::Concat(pieces.into_iter().map(|i| self.relativized(i, prefix)).collect()),
            Raw::Merge(stack) => Raw::Merge(stack.into_iter().map(|i| self.relativized(i, prefix)).collect()),
            Raw::MergeObj(stack) => Raw::MergeObj(stack.into_iter().map(|i| self.relativized(i, prefix)).collect()),
            _ => return id,
        };
        self.make(raw, at)
    }

    /// `ConfigConcatenation.concatenate`: none, one value, or a concatenation of the values left apart.
    fn concatenate(&mut self, pieces: Vec<Id>, at: Place) -> Result<Option<Id>, String> {
        let joined = self.consolidate(pieces, at)?;
        Ok(match joined.len() {
            0 => None,
            1 => Some(joined[0]),
            _ => Some(self.make(Raw::Concat(joined), at)),
        })
    }

    /// `ConfigConcatenation.consolidate`: the pieces flattened and joined left to right.
    fn consolidate(&mut self, pieces: Vec<Id>, at: Place) -> Result<Vec<Id>, String> {
        let mut out: Vec<Id> = Vec::with_capacity(pieces.len());
        for p in pieces {
            let flat = match self.raw(p) {
                Raw::Concat(inner) => inner.clone(),
                _ => vec![p],
            };
            for right in flat {
                match out.last().copied() {
                    None => out.push(right),
                    Some(left) => match self.join(left, right, at)? {
                        Some(j) => *out.last_mut().unwrap() = j,
                        None => out.push(right),
                    },
                }
            }
        }
        Ok(out)
    }

    /// `ConfigConcatenation.join`: two objects merged, the right winning; two lists concatenated; a list or an object
    /// beside an unquoted string the list or the object; none where either is unresolved; strings, numbers, booleans
    /// and `null` joined into a quoted string; a list or an object beside any of those an error at the left's place.
    fn join(&mut self, left: Id, right: Id, at: Place) -> Result<Option<Id>, String> {
        let list = |v: &Values, id: Id| matches!(v.raw(id), Raw::List(_));
        if self.is_object(left) && self.is_object(right) {
            return Ok(Some(self.with_fallback(right, left)));
        }
        if let (Raw::List(a), Raw::List(b)) = (self.raw(left), self.raw(right)) {
            let items = a.iter().chain(b).copied().collect();
            return Ok(Some(self.make(Raw::List(items), at)));
        }
        if (list(self, left) || self.is_object(left)) && matches!(self.raw(right), Raw::Str(_, false)) {
            return Ok(Some(left));
        }
        if self.is_unmergeable(left) || self.is_unmergeable(right) {
            return Ok(None);
        }
        match (self.text(left), self.text(right)) {
            (Some(a), Some(b)) => Ok(Some(self.make(Raw::Str(a + &b, true), self.at(left)))),
            _ => Err(format!("{}: a list or an object cannot be concatenated with a string, a number, a boolean or null: {} and {}", self.place(self.at(left)), self.render(left), self.render(right))),
        }
    }

    /// `transformToString`: a string, number, boolean or `null` as text.
    fn text(&self, id: Id) -> Option<String> {
        match self.raw(id) {
            Raw::Str(s, _) | Raw::Num(s) => Some(s.clone()),
            Raw::Bool(b) => Some(b.to_string()),
            Raw::Null => Some("null".to_string()),
            _ => None,
        }
    }

    /// A value as HOCON writes it, for messages.
    fn render(&self, id: Id) -> String {
        match self.raw(id) {
            Raw::Str(s, _) => quote(s),
            Raw::Num(s) => s.clone(),
            Raw::Bool(b) => b.to_string(),
            Raw::Null => "null".to_string(),
            Raw::List(items) => format!("[{}]", items.iter().map(|&i| self.render(i)).collect::<Vec<_>>().join(", ")),
            Raw::Obj(fields, _) => format!("{{{}}}", fields.iter().map(|(k, v)| format!("{}: {}", quote(k), self.render(*v))).collect::<Vec<_>>().join(", ")),
            Raw::Ref(path, optional, _) => substitution(path, *optional),
            Raw::Concat(pieces) => pieces.iter().map(|&i| self.render(i)).collect::<Vec<_>>().join(""),
            Raw::Merge(stack) | Raw::MergeObj(stack) => stack.iter().map(|&i| self.render(i)).collect::<Vec<_>>().join(" over "),
        }
    }

    fn place(&self, at: Place) -> String {
        format!("{}:{}:{}", self.files[at.file as usize], at.line, at.col)
    }
}

/// A JSON string.
fn quote(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    crate::watch::json_string(s, &mut out);
    out
}

/// `SubstitutionExpression.toString`: `${path}`, `${?path}`, an element quoted where it holds more than letters,
/// digits, `-` and `_` (`Path.render`).
fn substitution(path: &[String], optional: bool) -> String {
    let elements: Vec<String> = path.iter().map(|e| if e.is_empty() || e.chars().any(|c| !(c.is_alphanumeric() || c == '-' || c == '_')) { quote(e) } else { e.clone() }).collect();
    format!("${{{}{}}}", if optional { "?" } else { "" }, elements.join("."))
}

/// The files read, the values they hold, and the files being read now: an include of one of them is a cycle.
struct Loader<'a> {
    v: Values,
    read: &'a dyn Fn(&Path) -> std::io::Result<String>,
    open: Vec<PathBuf>,
}

/// What an `include` names (`ConfigIncludeKind`).
#[derive(Clone, Copy, PartialEq)]
enum IncludeKind {
    Heuristic,
    File,
    Url,
    Classpath,
}

impl Loader<'_> {
    /// The root object of the file `path` holding `text`.
    fn parse_file(&mut self, path: &Path, text: &str) -> Result<Id, String> {
        let file = self.v.files.len() as u32;
        self.v.files.push(path.to_string_lossy().into_owned());
        self.open.push(path.to_path_buf());
        let root = Parser { ld: self, file, dir: path.parent().map(Path::to_path_buf).unwrap_or_default(), b: text.as_bytes(), text, pos: 0, line: 1, col: 1, path_stack: Vec::new(), lists: 0 }.root();
        self.open.pop();
        root
    }

    /// The object an include reads (`SimpleIncluder`): `include "name"` beside the including file in `dir`, a name
    /// with no extension its `.conf` over its `.json`; `include file("name")` from the working directory, an empty
    /// object where missing unless `required`. A name typesafe-config would read as a URL, a missing file it would
    /// look for on the class path, and a `.properties` file are errors.
    fn include(&mut self, kind: IncludeKind, name: &str, required: bool, dir: &Path, at: &str) -> Result<Id, String> {
        let shown = |kind: IncludeKind| match kind {
            IncludeKind::Heuristic => quote(name),
            IncludeKind::File => format!("file({})", quote(name)),
            IncludeKind::Url => format!("url({})", quote(name)),
            IncludeKind::Classpath => format!("classpath({})", quote(name)),
        };
        let what = if required { format!("{}: `include required({})`", at, shown(kind)) } else { format!("{}: `include {}`", at, shown(kind)) };
        match kind {
            IncludeKind::Url => return Err(format!("{}: teq reads no URL", what)),
            IncludeKind::Classpath => return Err(format!("{}: teq reads no class path", what)),
            IncludeKind::File => {
                return match self.file(Path::new(name), &what)? {
                    Some(obj) => Ok(obj),
                    None if required => Err(format!("{}: there is no file {}", what, name)),
                    None => Ok(self.v.make(Raw::Obj(Vec::new(), false), Place { file: 0, line: 0, col: 0 })),
                };
            }
            IncludeKind::Heuristic => {}
        }
        if has_url_scheme(name) {
            return Err(format!("{}: teq reads no URL", what));
        }
        let beside = |n: &str| if Path::new(n).is_absolute() { PathBuf::from(n) } else { dir.join(n) };
        let missing = |p: &Path| format!("{}: there is no file {} (typesafe-config would look for it on the JVM's class path, which teq does not have)", what, p.display());
        if name.ends_with(".properties") {
            return Err(format!("{}: teq reads no Java properties file", what));
        }
        if name.ends_with(".conf") || name.ends_with(".json") {
            let p = beside(name);
            return self.file(&p, &what)?.ok_or_else(|| missing(&p));
        }
        let (conf, json) = (beside(&format!("{}.conf", name)), beside(&format!("{}.json", name)));
        if self.exists(&beside(&format!("{}.properties", name))) {
            return Err(format!("{}: teq reads no Java properties file ({}.properties)", what, name));
        }
        match (self.file(&conf, &what)?, self.file(&json, &what)?) {
            (Some(c), Some(j)) => Ok(self.v.with_fallback(c, j)),
            (Some(o), None) | (None, Some(o)) => Ok(o),
            (None, None) => Err(missing(&conf)),
        }
    }

    /// The root object of the file at `p`, none where there is no such file.
    fn file(&mut self, p: &Path, what: &str) -> Result<Option<Id>, String> {
        let text = match (self.read)(p) {
            Ok(t) => t,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(e) => return Err(format!("{}: cannot read {}: {}", what, p.display(), e)),
        };
        if self.open.iter().any(|o| same_file(o, p)) {
            return Err(format!("{}: {} is being read already, an include cycle", what, p.display()));
        }
        self.parse_file(p, &text).map(Some)
    }

    fn exists(&self, p: &Path) -> bool {
        !matches!((self.read)(p), Err(e) if e.kind() == std::io::ErrorKind::NotFound)
    }
}

fn same_file(a: &Path, b: &Path) -> bool {
    a == b || matches!((a.canonicalize(), b.canonicalize()), (Ok(x), Ok(y)) if x == y)
}

/// Whether `name` parses as a URL (`new URL(name)` in `SimpleIncluder.includeWithoutFallback`): a scheme Java knows.
fn has_url_scheme(name: &str) -> bool {
    let Some(i) = name.find(':') else { return false };
    matches!(name[..i].to_ascii_lowercase().as_str(), "file" | "http" | "https" | "jar" | "ftp" | "mailto")
}

/// A piece of a value as written on its line: a value, simple (a string, a number, a boolean, `null`, a substitution)
/// or not (a list, an object), or the whitespace between pieces.
enum Piece {
    Value(Id, bool),
    Space(String, Place),
}

struct Parser<'a, 'l> {
    ld: &'a mut Loader<'l>,
    file: u32,
    /// The directory of the file, which its includes' names are relative to.
    dir: PathBuf,
    b: &'a [u8],
    text: &'a str,
    pos: usize,
    line: u32,
    col: u32,
    /// The keys of the fields being read, outermost first (`ConfigParser`'s `pathStack`).
    path_stack: Vec<Vec<String>>,
    /// The lists being read (`arrayCount`).
    lists: usize,
}

impl Parser<'_, '_> {
    fn peek(&self) -> Option<u8> {
        self.b.get(self.pos).copied()
    }

    fn peek_at(&self, k: usize) -> Option<u8> {
        self.b.get(self.pos + k).copied()
    }

    /// Past `word`, which the text holds here.
    fn bump_word(&mut self, word: &str) {
        for _ in 0..word.len() {
            self.bump();
        }
    }

    fn bump(&mut self) -> Option<u8> {
        let c = self.peek()?;
        self.pos += 1;
        if c == b'\n' {
            self.line += 1;
            self.col = 1;
        } else if c & 0xc0 != 0x80 {
            self.col += 1;
        }
        Some(c)
    }

    fn here(&self) -> Place {
        Place { file: self.file, line: self.line, col: self.col }
    }

    fn place(&self, at: Place) -> String {
        self.ld.v.place(at)
    }

    fn error(&self, msg: &str) -> String {
        format!("{}: {}", self.place(self.here()), msg)
    }

    fn expect(&mut self, c: u8) -> Result<(), String> {
        if self.peek() == Some(c) {
            self.bump();
            Ok(())
        } else {
            Err(self.error(&format!("expected `{}`", c as char)))
        }
    }

    /// Skips spaces and comments, newlines too where `newlines`.
    fn skip_ws(&mut self, newlines: bool) {
        loop {
            match self.peek() {
                Some(b' ' | b'\t' | b'\r') => {
                    self.bump();
                }
                Some(b'\n') if newlines => {
                    self.bump();
                }
                Some(b'#') => self.skip_line(),
                Some(b'/') if self.peek_at(1) == Some(b'/') => self.skip_line(),
                _ => return,
            }
        }
    }

    fn skip_line(&mut self) {
        while self.peek().is_some_and(|c| c != b'\n') {
            self.bump();
        }
    }

    fn root(mut self) -> Result<Id, String> {
        self.skip_ws(true);
        let at = self.here();
        let root = if self.peek() == Some(b'{') {
            self.bump();
            let root = self.object(Some(b'}'), at)?;
            self.expect(b'}')?;
            root
        } else {
            self.object(None, at)?
        };
        self.skip_ws(true);
        if self.pos < self.b.len() {
            return Err(self.error("unexpected text after the configuration"));
        }
        Ok(root)
    }

    /// `ConfigParser.parseObject`: the fields up to `close` (`}`), or to the end of the text for the root; a key
    /// written again merges as `withFallback` merges it, a dotted key's value nested under its path
    /// (`createValueUnderPath`), an include's keys merged at its place.
    fn object(&mut self, close: Option<u8>, at: Place) -> Result<Id, String> {
        let mut values: Vec<(String, Id)> = Vec::new();
        loop {
            self.skip_ws(true);
            match self.peek() {
                None if close.is_none() => break,
                None => return Err(self.error("the object is not closed")),
                Some(c) if Some(c) == close => break,
                Some(b',') => {
                    self.bump();
                    continue;
                }
                _ => {}
            }
            if self.at_include() {
                self.include(&mut values)?;
                continue;
            }
            let path = self.key()?;
            self.skip_ws(false);
            let append = match self.peek() {
                Some(b'=' | b':') => {
                    self.bump();
                    self.skip_ws(true);
                    false
                }
                Some(b'+') if self.peek_at(1) == Some(b'=') => {
                    self.bump();
                    self.bump();
                    self.skip_ws(true);
                    true
                }
                Some(b'{') => false,
                _ => return Err(self.error("expected `=`, `:`, `+=` or `{` after a key")),
            };
            self.path_stack.push(path.clone());
            if append && self.lists > 0 {
                return Err(self.error("`+=` does not work inside a list (typesafe-config's limitation)"));
            }
            let mut value = self.value()?;
            if append {
                // `a += v` is `a = ${?a} [v]`.
                let at = self.ld.v.at(value);
                let previous = self.ld.v.make(Raw::Ref(self.path_stack.concat(), true, 0), at);
                let list = self.ld.v.make(Raw::List(vec![value]), at);
                value = self.ld.v.concatenate(vec![previous, list], at)?.expect("a concatenation of two values");
            }
            self.path_stack.pop();
            let key = path[0].clone();
            let mut new = value;
            for k in path[1..].iter().rev() {
                new = self.ld.v.make(Raw::Obj(vec![(k.clone(), new)], false), self.ld.v.at(value));
            }
            set(&mut self.ld.v, &mut values, key, new);
        }
        Ok(self.ld.v.make(Raw::Obj(values, false), at))
    }

    /// Whether an `include` starts here: the bare word alone, before whitespace, a quote or a character that ends
    /// bare text (`ConfigDocumentParser.isIncludeKeyword`).
    fn at_include(&self) -> bool {
        self.text[self.pos..].starts_with("include") && self.peek_at(7).is_none_or(|c| c.is_ascii_whitespace() || ends_bare(c))
    }

    /// `parseInclude`: `include "name"`, `include file("name")`, `url(...)`, `classpath(...)`, each also as
    /// `required(...)`; the included object's keys merged into `values`, each over the one before it, its
    /// substitutions prefixed with the path of the object it stands in.
    fn include(&mut self, values: &mut Vec<(String, Id)>) -> Result<(), String> {
        let at = self.here();
        self.bump_word("include");
        self.skip_ws(false);
        let required = self.text[self.pos..].starts_with("required(");
        if required {
            self.bump_word("required(");
            self.skip_ws(false);
        }
        let kinds = [("file(", IncludeKind::File), ("url(", IncludeKind::Url), ("classpath(", IncludeKind::Classpath)];
        let kind = match kinds.iter().find(|(w, _)| self.text[self.pos..].starts_with(w)) {
            Some(&(w, k)) => {
                self.bump_word(w);
                self.skip_ws(false);
                k
            }
            None => IncludeKind::Heuristic,
        };
        if self.peek() != Some(b'"') {
            return Err(self.error("`include` is followed by a quoted file name, `file(...)` or `required(...)`"));
        }
        let name = self.quoted()?;
        self.skip_ws(false);
        if kind != IncludeKind::Heuristic {
            self.expect(b')')?;
            self.skip_ws(false);
        }
        if required {
            self.expect(b')')?;
        }
        let place = self.place(at);
        let dir = self.dir.clone();
        let mut obj = self.ld.include(kind, &name, required, &dir, &place)?;
        if self.lists > 0 && !self.ld.v.resolved(obj) {
            return Err(format!("{}: an include inside a list cannot read substitutions (typesafe-config's limitation)", place));
        }
        if !self.path_stack.is_empty() {
            let prefix = self.path_stack.concat();
            obj = self.ld.v.relativized(obj, &prefix);
        }
        let Raw::Obj(fields, _) = self.ld.v.raw(obj).clone() else { unreachable!("an included file's root is an object") };
        for (k, v) in fields {
            set(&mut self.ld.v, values, k, v);
        }
        Ok(())
    }

    /// A key: names joined by dots, each bare or quoted.
    fn key(&mut self) -> Result<Vec<String>, String> {
        let mut parts = Vec::new();
        loop {
            let part = if self.peek() == Some(b'"') {
                self.quoted()?
            } else {
                let start = self.pos;
                while self.peek().is_some_and(|c| !matches!(c, b'.' | b'=' | b':' | b'{' | b' ' | b'\t' | b'\r' | b'\n' | b'#' | b'"' | b',' | b'}' | b'+')) {
                    self.bump();
                }
                if self.pos == start {
                    return Err(self.error("expected a key"));
                }
                self.text[start..self.pos].to_string()
            };
            parts.push(part);
            if self.peek() == Some(b'.') {
                self.bump();
            } else {
                return Ok(parts);
            }
        }
    }

    fn quoted(&mut self) -> Result<String, String> {
        if self.text[self.pos..].starts_with("\"\"\"") {
            return Err(self.error("triple-quoted strings are not supported"));
        }
        self.bump();
        let mut s = String::new();
        loop {
            match self.bump() {
                None | Some(b'\n') => return Err(self.error("the string is not closed")),
                Some(b'"') => return Ok(s),
                Some(b'\\') => match self.bump() {
                    Some(b'"') => s.push('"'),
                    Some(b'\\') => s.push('\\'),
                    Some(b'/') => s.push('/'),
                    Some(b'n') => s.push('\n'),
                    Some(b't') => s.push('\t'),
                    Some(b'r') => s.push('\r'),
                    Some(b'b') => s.push('\u{8}'),
                    Some(b'f') => s.push('\u{c}'),
                    Some(b'u') => {
                        let hex = self.text.get(self.pos..self.pos + 4).ok_or_else(|| self.error("a \\u escape needs four digits"))?;
                        let c = u32::from_str_radix(hex, 16).ok().and_then(char::from_u32).ok_or_else(|| self.error("a \\u escape needs four digits"))?;
                        for _ in 0..4 {
                            self.bump();
                        }
                        s.push(c);
                    }
                    _ => return Err(self.error("an unknown escape")),
                },
                Some(c) if c < 0x80 => s.push(c as char),
                Some(_) => {
                    // The rest of a multi-byte character: take it whole from the text.
                    let start = self.pos - 1;
                    let len = self.text[start..].chars().next().map_or(1, char::len_utf8);
                    s.push_str(&self.text[start..start + len]);
                    for _ in 1..len {
                        self.bump();
                    }
                }
            }
        }
    }

    /// A value: the pieces written on its line, up to its end, a comma, a closing bracket or a comment
    /// (`consolidateValues`), concatenated (`parseConcatenation`); the whitespace between two simple pieces is a piece
    /// of the value (`Tokenizer.WhitespaceSaver`), any other is not. A lone bare word is a number, a boolean, `null` or
    /// a string as it reads.
    fn value(&mut self) -> Result<Id, String> {
        let at = self.here();
        let mut pieces: Vec<Piece> = Vec::new();
        loop {
            let here = self.here();
            match self.peek() {
                Some(b' ' | b'\t' | b'\r') => {
                    let start = self.pos;
                    while matches!(self.peek(), Some(b' ' | b'\t' | b'\r')) {
                        self.bump();
                    }
                    pieces.push(Piece::Space(self.text[start..self.pos].to_string(), here));
                }
                Some(b'\n' | b',' | b']' | b'}' | b'#') | None => break,
                Some(b'/') if self.peek_at(1) == Some(b'/') => break,
                Some(b'{') => {
                    self.bump();
                    let obj = self.object(Some(b'}'), here)?;
                    self.expect(b'}')?;
                    pieces.push(Piece::Value(obj, false));
                }
                Some(b'[') => {
                    let list = self.list()?;
                    pieces.push(Piece::Value(list, false));
                }
                Some(b'$') if self.peek_at(1) == Some(b'{') => {
                    let r = self.substitution()?;
                    pieces.push(Piece::Value(r, true));
                }
                Some(b'"') => {
                    let s = self.quoted()?;
                    pieces.push(Piece::Value(self.ld.v.make(Raw::Str(s, true), here), true));
                }
                Some(c) if ends_bare(c) => return Err(self.error(&format!("`{}` is not allowed outside quotes", c as char))),
                Some(_) => {
                    let start = self.pos;
                    while self.peek().is_some_and(|c| !c.is_ascii_whitespace() && !ends_bare(c) && !matches!(c, b'"' | b',' | b']' | b'}' | b'#' | b'[' | b'{'))
                        && !(self.peek() == Some(b'/') && self.peek_at(1) == Some(b'/'))
                        && !(self.peek() == Some(b'$') && self.peek_at(1) == Some(b'{'))
                    {
                        self.bump();
                    }
                    let word = &self.text[start..self.pos];
                    let raw = match word {
                        "true" => Raw::Bool(true),
                        "false" => Raw::Bool(false),
                        "null" => Raw::Null,
                        w if w.parse::<f64>().is_ok() && w.starts_with(|c: char| c.is_ascii_digit() || c == '-') => Raw::Num(w.to_string()),
                        w => Raw::Str(w.to_string(), false),
                    };
                    pieces.push(Piece::Value(self.ld.v.make(raw, here), true));
                }
            }
        }
        let simple = |p: Option<&Piece>| matches!(p, Some(Piece::Value(_, true)));
        let mut values = Vec::new();
        for i in 0..pieces.len() {
            match &pieces[i] {
                Piece::Value(id, _) => values.push(*id),
                Piece::Space(s, here) => {
                    if i > 0 && simple(pieces.get(i - 1)) && simple(pieces.get(i + 1)) {
                        values.push(self.ld.v.make(Raw::Str(s.clone(), false), *here));
                    }
                }
            }
        }
        match values.len() {
            0 => Err(self.error("expected a value")),
            1 => Ok(values[0]),
            _ => {
                // Joined words and numbers are a string.
                let joined = self.ld.v.concatenate(values, at)?.expect("a concatenation of values");
                Ok(joined)
            }
        }
    }

    /// `[ ... ]`: values separated by commas or line ends.
    fn list(&mut self) -> Result<Id, String> {
        let at = self.here();
        self.bump();
        self.lists += 1;
        let mut items = Vec::new();
        loop {
            self.skip_ws(true);
            match self.peek() {
                Some(b']') => {
                    self.bump();
                    break;
                }
                Some(b',') => {
                    self.bump();
                }
                None => return Err(self.error("the list is not closed")),
                _ => items.push(self.value()?),
            }
        }
        self.lists -= 1;
        Ok(self.ld.v.make(Raw::List(items), at))
    }

    /// `${path}` or `${?path}` (`Tokenizer.pullSubstitution`): the path read as a key, the whitespace around it
    /// dropped and the whitespace within it kept (`PathParser.parsePathExpression`).
    fn substitution(&mut self) -> Result<Id, String> {
        let at = self.here();
        self.bump();
        self.bump();
        let optional = self.peek() == Some(b'?');
        if optional {
            self.bump();
        }
        let start = self.pos;
        while self.peek().is_some_and(|c| c != b'}' && c != b'\n') {
            if self.peek() == Some(b'"') {
                self.quoted()?;
            } else {
                self.bump();
            }
        }
        if self.peek() != Some(b'}') {
            return Err(format!("{}: `${{` is not closed with `}}`", self.place(at)));
        }
        let inner = &self.text[start..self.pos];
        self.bump();
        let path = path_of(inner).map_err(|e| format!("{}: {}", self.place(at), e))?;
        Ok(self.ld.v.make(Raw::Ref(path, optional, 0), at))
    }
}

/// Whether a bare word ends at `c`: a character typesafe-config reserves outside quotes, which it also refuses there.
fn ends_bare(c: u8) -> bool {
    matches!(c, b'$' | b'=' | b':' | b'+' | b'^' | b'?' | b'!' | b'@' | b'*' | b'&' | b'\\' | b'`')
}

/// The path of a substitution as written between `${` and `}`: elements split at the dots outside quotes, a quoted
/// part kept whole.
fn path_of(text: &str) -> Result<Vec<String>, String> {
    let text = text.trim_matches(|c: char| c == ' ' || c == '\t' || c == '\r');
    if text.is_empty() {
        return Err("a substitution needs a path".to_string());
    }
    let mut out = Vec::new();
    let mut cur = String::new();
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '"' => {
                for d in chars.by_ref() {
                    if d == '"' {
                        break;
                    }
                    cur.push(d);
                }
            }
            '.' => out.push(std::mem::take(&mut cur)),
            c => cur.push(c),
        }
    }
    out.push(cur);
    if out.iter().any(String::is_empty) {
        return Err(format!("the path `{}` has an empty element (a leading, trailing or doubled `.`)", text));
    }
    Ok(out)
}

/// Sets `key` in `values` to `v` over its value so far, where it has one (`v.withFallback(existing)`).
fn set(v: &mut Values, values: &mut Vec<(String, Id)>, key: String, new: Id) {
    match values.iter().position(|(k, _)| *k == key) {
        Some(i) => {
            let merged = v.with_fallback(new, values[i].1);
            values[i].1 = merged;
        }
        None => values.push((key, new)),
    }
}

/// A restriction of a resolution to the path under an object (`restrictToChild`).
type Restrict = Option<Rc<[String]>>;

/// `ResolveContext`: persistent, as Java's is, so that a reference catching a cycle continues with its context from
/// before the resolution that met it.
#[derive(Clone, Default)]
struct Ctx {
    memos: Rc<HashMap<(Id, Restrict), Id>>,
    restrict: Restrict,
    /// Every value being resolved, outermost first (`resolveStack`), for the message of a cycle.
    trace: Rc<Vec<Id>>,
    markers: Rc<Vec<Id>>,
}

impl Ctx {
    fn restrict(&self, to: Restrict) -> Ctx {
        Ctx { restrict: to, ..self.clone() }
    }

    fn unrestricted(&self) -> Ctx {
        self.restrict(None)
    }

    fn with_marker(&self, id: Id, on: bool) -> Ctx {
        let mut markers = (*self.markers).clone();
        if on {
            markers.push(id);
        } else {
            markers.retain(|&m| m != id);
        }
        Ctx { markers: Rc::new(markers), ..self.clone() }
    }

    fn memoize(&self, key: (Id, Restrict), v: Id) -> Ctx {
        let mut memos = (*self.memos).clone();
        memos.insert(key, v);
        Ctx { memos: Rc::new(memos), ..self.clone() }
    }

    fn traced(&self, id: Id, push: bool) -> Ctx {
        let mut trace = (*self.trace).clone();
        if push {
            trace.push(id);
        } else {
            trace.pop();
        }
        Ctx { trace: Rc::new(trace), ..self.clone() }
    }
}

/// `ResolveSource`: the root substitutions are looked up in, and the containers from the one being resolved up to it
/// (`pathFromRoot`, the innermost first), where the resolution came down from the root.
#[derive(Clone)]
struct Source {
    root: Id,
    path: Option<Rc<Vec<Id>>>,
}

impl Source {
    fn new(root: Id) -> Source {
        Source { root, path: None }
    }

    fn push_parent(&self, parent: Id) -> Source {
        match &self.path {
            None if parent == self.root => Source { root: self.root, path: Some(Rc::new(vec![parent])) },
            None => self.clone(),
            Some(path) => {
                let mut p = vec![parent];
                p.extend(path.iter());
                Source { root: self.root, path: Some(Rc::new(p)) }
            }
        }
    }
}

/// What stops a resolution: a cycle (`NotPossibleToResolve`, with the context it was met in) or an error.
enum Fail {
    Cycle(Ctx),
    Error(String),
}

type Resolved = Result<(Ctx, Option<Id>), Fail>;

struct Resolver<'a> {
    v: &'a mut Values,
    env: &'a dyn Fn(&str) -> Option<String>,
}

impl Resolver<'_> {
    fn error(&self, at: Place, msg: &str) -> Fail {
        Fail::Error(format!("{}: {}", self.v.place(at), msg))
    }

    /// `ResolveContext.resolve(root, root, options)`, the files' names put in its errors' places.
    fn resolve_root(&mut self, root: Id) -> Result<Id, String> {
        match self.resolve(Ctx::default(), root, &Source::new(root)) {
            Ok((_, Some(v))) => Ok(v),
            Ok((_, None)) => Ok(self.v.make(Raw::Obj(Vec::new(), false), self.v.at(root))),
            Err(Fail::Error(e)) => Err(e),
            Err(Fail::Cycle(_)) => Err("teq fix: a cycle of substitutions no substitution caught".to_string()),
        }
    }

    fn resolve(&mut self, ctx: Ctx, id: Id, source: &Source) -> Resolved {
        let (c, v) = self.real_resolve(ctx.traced(id, true), id, source)?;
        Ok((c.traced(id, false), v))
    }

    /// `realResolve`: a memo under no restriction, or under this one; a value marked as being resolved a cycle;
    /// else the value's resolution, memoized.
    fn real_resolve(&mut self, ctx: Ctx, id: Id, source: &Source) -> Resolved {
        let full = (id, None);
        if let Some(&v) = ctx.memos.get(&full) {
            return Ok((ctx, Some(v)));
        }
        let restricted = ctx.restrict.clone().map(|r| (id, Some(r)));
        if let Some(&v) = restricted.as_ref().and_then(|k| ctx.memos.get(k)) {
            return Ok((ctx, Some(v)));
        }
        if ctx.markers.contains(&id) {
            return Err(Fail::Cycle(ctx));
        }
        let (c, v) = self.substitutions(ctx, id, source)?;
        let c = match v {
            None => c,
            Some(r) if self.v.resolved(r) => c.memoize(full, r),
            Some(r) => match restricted {
                Some(k) => c.memoize(k, r),
                None => return Err(self.error(self.v.at(id), "teq fix: a value left unresolved by a full resolution")),
            },
        };
        Ok((c, v))
    }

    /// `resolveSubstitutions` of each kind of value.
    fn substitutions(&mut self, ctx: Ctx, id: Id, source: &Source) -> Resolved {
        if self.v.resolved(id) {
            return Ok((ctx, Some(id)));
        }
        match self.v.raw(id).clone() {
            Raw::Obj(fields, ignores) => self.object(ctx, id, fields, ignores, source),
            Raw::List(items) => {
                if ctx.restrict.is_some() {
                    return Ok((ctx, Some(id)));
                }
                let source = source.push_parent(id);
                let mut c = ctx;
                let mut out = Vec::with_capacity(items.len());
                for &i in &items {
                    let (c2, r) = self.resolve(c, i, &source)?;
                    c = c2;
                    out.extend(r);
                }
                if out == items {
                    return Ok((c, Some(id)));
                }
                let at = self.v.at(id);
                Ok((c, Some(self.v.make(Raw::List(out), at))))
            }
            Raw::Ref(path, optional, prefix) => self.reference(ctx, id, &path, optional, prefix, source),
            Raw::Concat(pieces) => {
                let mut c = ctx;
                let mut out = Vec::with_capacity(pieces.len());
                for p in pieces {
                    // A string needs its pieces whole: the restriction lifted for each.
                    let restriction = c.restrict.clone();
                    let (c2, r) = self.resolve(c.unrestricted(), p, source)?;
                    c = c2.restrict(restriction);
                    out.extend(r);
                }
                let at = self.v.at(id);
                let joined = self.v.consolidate(out, at).map_err(Fail::Error)?;
                match joined[..] {
                    [] => Ok((c, None)),
                    [one] => Ok((c, Some(one))),
                    _ => Err(self.error(at, "teq fix: a concatenation that joined to several values")),
                }
            }
            Raw::Merge(stack) | Raw::MergeObj(stack) => self.merge(ctx, id, &stack, source),
            Raw::Str(..) | Raw::Num(_) | Raw::Bool(_) | Raw::Null => Ok((ctx, Some(id))),
        }
    }

    /// `SimpleConfigObject.resolveSubstitutions` with its `ResolveModifier`: under a restriction only the field on its
    /// path, and that one restricted to the rest of it (the path's last field itself left as it is); else every field.
    /// A field resolved to nothing (a missing `${?x}`) is removed.
    fn object(&mut self, ctx: Ctx, id: Id, fields: Vec<(String, Id)>, ignores: bool, source: &Source) -> Resolved {
        let source = source.push_parent(id);
        let original = ctx.restrict.clone();
        let mut c = ctx;
        let mut changed = false;
        let mut out: Vec<(String, Id)> = Vec::with_capacity(fields.len());
        for (k, v) in fields {
            let modified = match &original {
                Some(r) if r[0] != k || r.len() == 1 => Some(v),
                Some(r) => {
                    let rest: Rc<[String]> = r[1..].into();
                    let (c2, m) = self.resolve(c.restrict(Some(rest)), v, &source)?;
                    c = c2.restrict(original.clone());
                    m
                }
                None => {
                    let (c2, m) = self.resolve(c.unrestricted(), v, &source)?;
                    c = c2.restrict(original.clone());
                    m
                }
            };
            changed |= modified != Some(v);
            if let Some(m) = modified {
                out.push((k, m));
            }
        }
        if !changed {
            return Ok((c, Some(id)));
        }
        let at = self.v.at(id);
        Ok((c, Some(self.v.make(Raw::Obj(out, ignores), at))))
    }

    /// `ConfigReference.resolveSubstitutions`: the path looked up from the root, the value found resolved against the
    /// root it was found in; a cycle met an error here unless the reference is optional, its value then nothing; a
    /// required reference that finds nothing an error.
    fn reference(&mut self, ctx: Ctx, id: Id, path: &[String], optional: bool, prefix: usize, source: &Source) -> Resolved {
        let marked = ctx.with_marker(id, true);
        let mut current = marked.clone();
        let found = (|| -> Resolved {
            let (c, v, from) = self.lookup(marked.clone(), path, prefix, source, self.v.at(id))?;
            current = c.clone();
            match v {
                Some(v) => {
                    let root = *from.last().expect("a path from the root");
                    self.resolve(c, v, &Source { root, path: Some(Rc::new(from)) })
                }
                None => Ok((c, None)),
            }
        })();
        let at = self.v.at(id);
        let (c, v) = match found {
            Ok(r) => r,
            Err(Fail::Cycle(_)) if optional => (current, None),
            Err(Fail::Cycle(e)) => {
                let trace: Vec<String> = e.trace.iter().filter_map(|&t| match self.v.raw(t) {
                    Raw::Ref(p, o, _) => Some(substitution(p, *o)),
                    _ => None,
                }).collect();
                return Err(self.error(at, &format!("the substitution {} is part of a cycle of substitutions: {}", substitution(path, optional), trace.join(", "))));
            }
            Err(e) => return Err(e),
        };
        if v.is_none() && !optional {
            return Err(self.error(at, &format!("the substitution {} has no value", substitution(path, optional))));
        }
        Ok((c.with_marker(id, false), v))
    }

    /// `ResolveSource.lookupSubst`: the path from the root, then without the prefix an include in an object gave it,
    /// then the environment's variable of that name, which takes the place `at` of the substitution, no file holding
    /// it; with the containers from the value's up to its root.
    fn lookup(&mut self, ctx: Ctx, path: &[String], prefix: usize, source: &Source, at: Place) -> Result<(Ctx, Option<Id>, Vec<Id>), Fail> {
        let (mut c, mut v, mut from) = self.find(source.root, ctx.clone(), path)?;
        if v.is_none() {
            let unprefixed = &path[prefix..];
            if prefix > 0 {
                (c, v, from) = self.find(source.root, c, unprefixed)?;
            }
            if v.is_none() {
                if let [name] = unprefixed {
                    if let Some(value) = (self.env)(name) {
                        let s = self.v.make(Raw::Str(value, true), at);
                        let env = self.v.make(Raw::Obj(vec![(name.clone(), s)], false), at);
                        return Ok((ctx, Some(s), vec![env]));
                    }
                }
            }
        }
        Ok((c, v, from))
    }

    /// `findInObject`: `obj` resolved as far as the path needs, then walked; with the containers walked, innermost
    /// first.
    fn find(&mut self, obj: Id, ctx: Ctx, path: &[String]) -> Result<(Ctx, Option<Id>, Vec<Id>), Fail> {
        let restriction = ctx.restrict.clone();
        let (c, partial) = self.resolve(ctx.restrict(Some(path.into())), obj, &Source::new(obj))?;
        let c = c.restrict(restriction);
        let mut node = partial.expect("an object resolved to nothing");
        let mut from = Vec::new();
        for (i, key) in path.iter().enumerate() {
            from.insert(0, node);
            let v = self.peek(node, key)?;
            if i + 1 == path.len() {
                return Ok((c, v, from));
            }
            match v {
                Some(next) if self.v.is_object(next) => node = next,
                _ => return Ok((c, None, from)),
            }
        }
        unreachable!("a path of no element")
    }

    /// `attemptPeekWithPartialResolve`: an object's field; a delayed merge of objects the first layer's field that
    /// takes no fallbacks, none where a list hides the rest, an error where an unresolved layer may hide it.
    fn peek(&self, obj: Id, key: &str) -> Result<Option<Id>, Fail> {
        match self.v.raw(obj) {
            Raw::Obj(..) => Ok(self.v.field(obj, key)),
            Raw::MergeObj(stack) => {
                for &layer in stack {
                    match self.v.raw(layer) {
                        Raw::Obj(..) => match self.v.field(layer, key) {
                            Some(v) if self.v.ignores_fallbacks(v) => return Ok(Some(v)),
                            _ => continue,
                        },
                        _ if self.v.is_unmergeable(layer) => return Err(self.error(self.v.at(obj), &format!("the key `{}` is not available before the value it may stand in is resolved", key))),
                        _ => return Ok(None),
                    }
                }
                Ok(None)
            }
            _ => Ok(None),
        }
    }

    /// `ConfigDelayedMerge.resolveSubstitutions`: each value of the stack resolved and merged under the ones before it;
    /// an unresolved one against a root where the stack stands as the values below that one merged
    /// (`makeReplacement`), so that a self-reference sees the value before its own assignment.
    fn merge(&mut self, ctx: Ctx, id: Id, stack: &[Id], source: &Source) -> Resolved {
        let mut c = ctx;
        let mut merged: Option<Id> = None;
        for (count, &end) in stack.iter().enumerate() {
            let for_end = if self.v.is_unmergeable(end) {
                let remainder = self.replacement(&stack[count + 1..]);
                self.replace_within_parent(source, id, remainder)?.reset()
            } else {
                source.push_parent(id)
            };
            let (c2, r) = self.resolve(c, end, &for_end)?;
            c = c2;
            if let Some(r) = r {
                merged = Some(match merged {
                    None => r,
                    Some(m) => self.v.with_fallback(m, r),
                });
            }
        }
        Ok((c, merged))
    }

    /// `makeReplacement`: the values merged, none for none.
    fn replacement(&mut self, rest: &[Id]) -> Option<Id> {
        let (&first, others) = rest.split_first()?;
        Some(others.iter().fold(first, |m, &v| self.v.with_fallback(m, v)))
    }

    /// `replaceWithinCurrentParent`: the source with `old` in the container it stands in replaced, and every container
    /// up to the root made anew around it.
    fn replace_within_parent(&mut self, source: &Source, old: Id, new: Option<Id>) -> Result<Source, Fail> {
        if Some(old) == new {
            return Ok(source.clone());
        }
        match &source.path {
            Some(path) => {
                let parent = path[0];
                let replaced = self.v.replace_child(parent, old, new).filter(|&p| self.v.is_container(p));
                Ok(self.replace_parent(source, parent, replaced))
            }
            None if old == source.root && new.is_some_and(|n| self.v.is_container(n)) => Ok(Source::new(self.root_object(new.unwrap()))),
            None => Err(self.error(self.v.at(old), "teq fix: a delayed merge outside the configuration's root")),
        }
    }

    /// `replaceCurrentParent`.
    fn replace_parent(&mut self, source: &Source, old: Id, new: Option<Id>) -> Source {
        if Some(old) == new {
            return source.clone();
        }
        let path = source.path.as_ref().expect("a path from the root");
        match self.replace(path, old, new) {
            Some(p) => Source { root: *p.last().unwrap(), path: Some(Rc::new(p)) },
            None => Source::new(self.v.make(Raw::Obj(Vec::new(), false), self.v.at(source.root))),
        }
    }

    /// `ResolveSource.replace`: the containers from `old` up, `old` replaced by `new` and each one above it by itself
    /// with the one below replaced; none where the root itself goes.
    fn replace(&mut self, list: &[Id], old: Id, new: Option<Id>) -> Option<Vec<Id>> {
        debug_assert_eq!(list[0], old);
        let parent = list.get(1).copied();
        match (new.filter(|&n| self.v.is_container(n)), parent) {
            (None, None) => None,
            (None, Some(p)) => {
                let replaced = self.v.replace_child(p, old, None);
                self.replace(&list[1..], p, replaced)
            }
            (Some(n), None) => Some(vec![n]),
            (Some(n), Some(p)) => {
                let replaced = self.v.replace_child(p, old, Some(n));
                let mut out = vec![n];
                if let Some(tail) = self.replace(&list[1..], p, replaced) {
                    out.extend(tail);
                }
                Some(out)
            }
        }
    }

    fn root_object(&mut self, v: Id) -> Id {
        if self.v.is_object(v) {
            v
        } else {
            self.v.make(Raw::Obj(Vec::new(), false), self.v.at(v))
        }
    }
}

impl Source {
    fn reset(self) -> Source {
        Source { root: self.root, path: None }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(file: &str, text: &str) -> Result<Conf, String> {
        load_files(file, &[(file, text)], &[])
    }

    /// The configuration `main` over the files `files` (path, text) and the environment `env`.
    fn load_files(main: &str, files: &[(&str, &str)], env: &[(&str, &str)]) -> Result<Conf, String> {
        let files: HashMap<PathBuf, String> = files.iter().map(|(p, t)| (PathBuf::from(p), t.to_string())).collect();
        let env: HashMap<String, String> = env.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect();
        let read = |p: &Path| files.get(p).cloned().ok_or_else(|| std::io::Error::from(std::io::ErrorKind::NotFound));
        load_with(Path::new(main), &read, &|k: &str| env.get(k).cloned())
    }

    /// The configuration as typesafe-config's concise JSON renders it, its keys sorted.
    fn json(c: &Conf, n: u32) -> String {
        match &c.nodes[n as usize].value {
            Value::Str(s) => quote(s),
            Value::Num(s) => s.clone(),
            Value::Bool(b) => b.to_string(),
            Value::Null => "null".to_string(),
            Value::List(items) => format!("[{}]", items.iter().map(|&i| json(c, i)).collect::<Vec<_>>().join(",")),
            Value::Obj(fields) => {
                let mut fields = fields.clone();
                fields.sort_by(|a, b| a.0.cmp(&b.0));
                format!("{{{}}}", fields.iter().map(|(k, v)| format!("{}:{}", quote(k), json(c, *v))).collect::<Vec<_>>().join(","))
            }
        }
    }

    fn resolved(main: &str, files: &[(&str, &str)]) -> String {
        match load_files(main, files, &[]) {
            Ok(c) => json(&c, 0),
            Err(e) => format!("error: {}", e),
        }
    }

    #[test]
    fn scalafix_shapes() {
        let text = "rules = [\n  DisableSyntax\n  \"file:x/NoDummy.scala\",\n]\nDisableSyntax.noVars = true\nDisableSyntax {\n  noNulls: true # comment\n  regex = [\n    \"print\"\"ln\"\n    { id = offensive, pattern = \"[Pp]imp\" }\n  ]\n}\n// a comment\nlint.error.includes = \".*\"\nn = 42\n";
        let c = parse("c.conf", text).unwrap();
        assert_eq!(c.strings(c.at(&["rules"]).unwrap()).unwrap(), ["DisableSyntax", "file:x/NoDummy.scala"]);
        let ds = c.at(&["DisableSyntax"]).unwrap();
        assert!(matches!(c.nodes[c.field(ds, "noVars").unwrap() as usize].value, Value::Bool(true)));
        assert!(matches!(c.nodes[c.field(ds, "noNulls").unwrap() as usize].value, Value::Bool(true)));
        let Value::List(regex) = &c.nodes[c.field(ds, "regex").unwrap() as usize].value else { panic!() };
        assert_eq!(c.str(regex[0]).as_deref(), Some("println"));
        assert_eq!(c.str(c.field(regex[1], "pattern").unwrap()).as_deref(), Some("[Pp]imp"));
        assert_eq!(c.str(c.at(&["lint", "error", "includes"]).unwrap()).as_deref(), Some(".*"));
        assert!(matches!(&c.nodes[c.at(&["n"]).unwrap() as usize].value, Value::Num(n) if n == "42"));
        assert_eq!(c.place(c.at(&["DisableSyntax", "noVars"]).unwrap()), "c.conf:5:24");
    }

    #[test]
    fn quoted_parts_keep_their_characters() {
        // HOCON's concatenation: the quoted parts as written, what stands between them kept, what stands outside not.
        let c = parse("c", "a = \" var \"\nb = \" x \" \"y\"\nc = x \"y\" z  \nd = [\" v \", w ]\n").unwrap();
        assert_eq!(c.str(c.at(&["a"]).unwrap()).as_deref(), Some(" var "));
        assert_eq!(c.str(c.at(&["b"]).unwrap()).as_deref(), Some(" x  y"));
        assert_eq!(c.str(c.at(&["c"]).unwrap()).as_deref(), Some("x y z"));
        assert_eq!(c.strings(c.at(&["d"]).unwrap()).unwrap(), [" v ", "w"]);
    }

    #[test]
    fn unsupported_is_an_error_at_its_place() {
        assert!(parse("c", "a = \"\"\"x\"\"\"\n").unwrap_err().contains("triple-quoted"));
        assert!(parse("c", "a = [1, 2\n").unwrap_err().contains("not closed"));
        assert_eq!(parse("c", "include url(\"file:///x.conf\")\n").unwrap_err(), "c:1:1: `include url(\"file:///x.conf\")`: teq reads no URL");
        assert_eq!(parse("c", "include classpath(\"x.conf\")\n").unwrap_err(), "c:1:1: `include classpath(\"x.conf\")`: teq reads no class path");
        assert_eq!(parse("c", "a = b*c\n").unwrap_err(), "c:1:6: `*` is not allowed outside quotes");
    }

    // The cases of typesafe-config 1.4.6's own reading (`ConfigFactory.parseFile(f).resolve()`, rendered concise), each
    // expected string the JSON it gave; the record's messages are typesafe-config's, these teq's at the same place.

    const DS_BASE: &str = "rules = [DisableSyntax]\nDisableSyntax.noVars = true\nDisableSyntax.regex = [\n  { id = \"a\", pattern = \"A\", message = \"ma\" }\n]\n";
    const DS_JSON: &str = r#"{"DisableSyntax":{"noVars":true,"regex":[{"id":"a","message":"ma","pattern":"A"},{"id":"x","message":"mx","pattern":"X"}]},"rules":["DisableSyntax"]}"#;

    #[test]
    fn include_and_self_append() {
        let app = "include \".scalafix.conf\"\nDisableSyntax.regex = ${DisableSyntax.regex} [\n  { id = \"x\", pattern = \"X\", message = \"mx\" }\n]\n";
        assert_eq!(resolved("d/app.conf", &[("d/app.conf", app), ("d/.scalafix.conf", DS_BASE)]), DS_JSON);
        let c = load_files("d/app.conf", &[("d/app.conf", app), ("d/.scalafix.conf", DS_BASE)], &[]).unwrap();
        let Value::List(regex) = &c.nodes[c.at(&["DisableSyntax", "regex"]).unwrap() as usize].value else { panic!() };
        assert_eq!(c.place(c.field(regex[0], "id").unwrap()), "d/.scalafix.conf:4:10");
        assert_eq!(c.place(c.field(regex[1], "id").unwrap()), "d/app.conf:3:10");
        assert_eq!(c.place(c.at(&["DisableSyntax", "noVars"]).unwrap()), "d/.scalafix.conf:2:24");
        // t31: the same as a nested object; t29: written before the include, the include's list replaces it.
        let nested = "include \".scalafix.conf\"\nDisableSyntax {\n  regex = ${DisableSyntax.regex} [ { id = \"y\" } ]\n}\n";
        assert_eq!(resolved("d/app.conf", &[("d/app.conf", nested), ("d/.scalafix.conf", DS_BASE)]), r#"{"DisableSyntax":{"noVars":true,"regex":[{"id":"a","message":"ma","pattern":"A"},{"id":"y"}]},"rules":["DisableSyntax"]}"#);
        let before = "DisableSyntax.regex = ${DisableSyntax.regex} [ 1 ]\ninclude \".scalafix.conf\"\n";
        assert_eq!(resolved("d/app.conf", &[("d/app.conf", before), ("d/.scalafix.conf", DS_BASE)]), r#"{"DisableSyntax":{"noVars":true,"regex":[{"id":"a","message":"ma","pattern":"A"}]},"rules":["DisableSyntax"]}"#);
    }

    #[test]
    fn substitutions() {
        let one = |text: &str| resolved("c.conf", &[("c.conf", text)]);
        assert_eq!(one("p = [\"a.\"]\nq = ${p}\np = [\"b.\"]\n"), r#"{"p":["b."],"q":["b."]}"#);
        assert_eq!(one("a = ${?missing}\nb = 1\nk = 1\nk = ${?missing}\n"), r#"{"b":1,"k":1}"#);
        assert_eq!(one("a = ${?missing} [1]\ns = \"x\" ${?missing}\nt = ${?missing} ${?missing2}\n"), r#"{"a":[1],"s":"x ","t":" "}"#);
        assert_eq!(one("b = 1\na = ${missing}\n"), "error: c.conf:2:5: the substitution ${missing} has no value");
        assert_eq!(one("a = ${b}\nb = ${a}\n"), "error: c.conf:2:5: the substitution ${a} is part of a cycle of substitutions: ${b}, ${a}, ${b}");
        // typesafe-config resolves the fields in its hash map's order and so names this cycle from `a`, at line 1; teq
        // resolves them in the file's order.
        assert_eq!(one("b = ${a}\na = ${b}\n"), "error: c.conf:2:5: the substitution ${b} is part of a cycle of substitutions: ${a}, ${b}, ${a}");
        assert_eq!(one("zz = ${m1}\naa = ${m2}\n"), "error: c.conf:1:6: the substitution ${m1} has no value");
        assert_eq!(one("a = ${a} [1]\n"), "error: c.conf:1:5: the substitution ${a} is part of a cycle of substitutions: ${a}, ${a}");
        assert_eq!(one("a = ${?a} [1]\n"), r#"{"a":[1]}"#);
        assert_eq!(one("a += 1\na += 2\n"), r#"{"a":[1,2]}"#);
        assert_eq!(one("a = [0]\na = ${a} [1]\na = ${a} [2]\n"), r#"{"a":[0,1,2]}"#);
        assert_eq!(one("p = [1]\nq = [2]\nt = ${p} ${q}\nstr = \"s\"\nu = ${str} ${str}\nv = ${str}${str}\nw = a ${str} b\n"), r#"{"p":[1],"q":[2],"str":"s","t":[1,2],"u":"s s","v":"ss","w":"a s b"}"#);
        assert_eq!(one("o = {a: 1} {b: 2}\no2 = ${o} {c: 3}\no3 = {a: 9}\no3 = ${o}\n"), r#"{"o":{"a":1,"b":2},"o2":{"a":1,"b":2,"c":3},"o3":{"a":1,"b":2}}"#);
        assert_eq!(one("a = ${}\n"), "error: c.conf:1:5: a substitution needs a path");
        assert_eq!(one("a = ${a.b} [1]\na = 1\n"), r#"{"a":1}"#);
        assert_eq!(one("a { x = 1 }\na = ${?nope}\nb = ${a}\nc { y = ${a.x} }\n"), r#"{"a":{"x":1},"b":{"x":1},"c":{"y":1}}"#);
        assert_eq!(one("a { b = ${a.c}, c = 1 }\n"), r#"{"a":{"b":1,"c":1}}"#);
        assert_eq!(one("a = {x: 1}\na = 1\na = {y: 2}\nb = 1\nb.c = 2\nd.e = 1\nd.f = 2\n"), r#"{"a":{"y":2},"b":{"c":2},"d":{"e":1,"f":2}}"#);
        assert_eq!(one("\"a b\" = 1\nc = ${ a b }\nd = ${\"a b\"}\n"), r#"{"a b":1,"c":1,"d":1}"#);
    }

    #[test]
    fn concatenations() {
        let one = |text: &str| resolved("c.conf", &[("c.conf", text)]);
        // The error at the left value's place, the referenced one's for a substitution (typesafe-config's).
        assert_eq!(one("a = [1] \"x\"\n"), "error: c.conf:1:5: a list or an object cannot be concatenated with a string, a number, a boolean or null: [1] and \"x\"");
        assert_eq!(one("l = [1]\na = ${l} \"x\"\n"), "error: c.conf:1:5: a list or an object cannot be concatenated with a string, a number, a boolean or null: [1] and \" x\"");
        assert_eq!(one("s = \"str\"\na = ${s} [1]\n"), "error: c.conf:1:5: a list or an object cannot be concatenated with a string, a number, a boolean or null: \"str\" and [1]");
        assert_eq!(one("o = {x: 1}\na = ${o} [1]\n"), "error: c.conf:1:5: a list or an object cannot be concatenated with a string, a number, a boolean or null: {\"x\": 1} and [1]");
        assert_eq!(one("a = [1] x\n"), r#"{"a":[1]}"#);
        assert_eq!(one("b = [1] ${?missing}\n"), r#"{"b":[1]}"#);
        assert_eq!(one("c = [1] \"  \"\n"), "error: c.conf:1:5: a list or an object cannot be concatenated with a string, a number, a boolean or null: [1] and \"  \"");
    }

    #[test]
    fn includes() {
        assert_eq!(resolved("t/main.conf", &[("t/main.conf", "include \"sub/a.conf\"\nm = 0\n"), ("t/sub/a.conf", "include \"b.conf\"\na = 1\n"), ("t/sub/b.conf", "b = 2\n"), ("t/b.conf", "b = 99\n")]), r#"{"a":1,"b":2,"m":0}"#);
        assert_eq!(resolved("c.conf", &[("c.conf", "x = 0\ny { f = 2 }\ninclude \"inc.conf\"\na = 2\nb.c = 3\n"), ("inc.conf", "a = 1\nb { c = 1, d = 2 }\nx = 5\ny { e = 1 }\n")]), r#"{"a":2,"b":{"c":3,"d":2},"x":5,"y":{"e":1,"f":2}}"#);
        assert_eq!(resolved("c.conf", &[("c.conf", "top = 7\nn { include \"inc2.conf\" }\n"), ("inc2.conf", "x = 1\ny = ${x}\nz = ${top}\n")]), r#"{"n":{"x":1,"y":1,"z":7},"top":7}"#);
        assert_eq!(resolved("c.conf", &[("c.conf", "include \"inc.conf\"\np = [2]\n"), ("inc.conf", "p = [1]\nq = ${p}\n")]), r#"{"p":[2],"q":[2]}"#);
        assert_eq!(resolved("c.conf", &[("c.conf", "include \"inc.conf\"\np = 3\n"), ("inc.conf", "q = ${p}\nr = ${?p}\n")]), r#"{"p":3,"q":3,"r":3}"#);
        assert_eq!(resolved("c.conf", &[("c.conf", "l = [1]\nl += 2\nm { include \"inc.conf\" }\nn = ${m.v}\n"), ("inc.conf", "v = ${?w} [3]\nw = [4]\n")]), r#"{"l":[1,2],"m":{"v":[4,3],"w":[4]},"n":[4,3]}"#);
        assert_eq!(resolved("c.conf", &[("c.conf", "include \"x\"\n"), ("x.conf", "k = conf\nonly_conf = 1\n"), ("x.json", "{ \"k\": \"json\", \"only_json\": 2 }\n")]), r#"{"k":"conf","only_conf":1,"only_json":2}"#);
        assert_eq!(resolved("c.conf", &[("c.conf", "include file(\"nope.conf\")\na = 1\n")]), r#"{"a":1}"#);
        // The errors: a missing included file (typesafe-config would ask the class path), a required one, a Java
        // properties file, a cycle, an include in a list reading substitutions, an unquoted name.
        assert_eq!(resolved("c.conf", &[("c.conf", "include \"nope.conf\"\na = 1\n")]), "error: c.conf:1:1: `include \"nope.conf\"`: there is no file nope.conf (typesafe-config would look for it on the JVM's class path, which teq does not have)");
        assert_eq!(resolved("c.conf", &[("c.conf", "include required(file(\"nope.conf\"))\na = 1\n")]), "error: c.conf:1:1: `include required(file(\"nope.conf\"))`: there is no file nope.conf");
        assert_eq!(resolved("c.conf", &[("c.conf", "include \"p\"\na = 1\n"), ("p.properties", "k=v\n")]), "error: c.conf:1:1: `include \"p\"`: teq reads no Java properties file (p.properties)");
        assert_eq!(resolved("c.conf", &[("c.conf", "include \"c.conf\"\na = 1\n")]), "error: c.conf:1:1: `include \"c.conf\"`: c.conf is being read already, an include cycle");
        assert_eq!(resolved("c.conf", &[("c.conf", "l = [ { include \"inc.conf\" } ]\n"), ("inc.conf", "x = ${y}\ny = 1\n")]), "error: c.conf:1:9: an include inside a list cannot read substitutions (typesafe-config's limitation)");
        assert_eq!(resolved("c.conf", &[("c.conf", "include = 1\n")]), "error: c.conf:1:9: `include` is followed by a quoted file name, `file(...)` or `required(...)`");
        // A missing reference in an included file, at its place there.
        assert_eq!(resolved("c.conf", &[("c.conf", "include \"inc.conf\"\na = 1\n"), ("inc.conf", "ok = 1\nx = ${missing}\n")]), "error: inc.conf:2:5: the substitution ${missing} has no value");
    }

    #[test]
    fn environment() {
        let c = load_files("c.conf", &[("c.conf", "h = ${?HOME}\nu = ${USER}\nz = ${?NO_SUCH_VAR_TEQ}\n")], &[("HOME", "/home/u"), ("USER", "u")]).unwrap();
        assert_eq!(json(&c, 0), r#"{"h":"/home/u","u":"u"}"#);
    }
}
