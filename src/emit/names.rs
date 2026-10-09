use crate::intern::{FxHasher, FxMap, Interner};
use crate::source::{FileId, SourceFile};
use crate::symbols::*;
use crate::tir::Program;
use crate::types::*;
use std::hash::Hasher;

pub(super) const RESERVED: &[&str] = &[
    "arguments", "await", "break", "case", "catch", "class", "const", "continue", "debugger",
    "default", "delete", "do", "else", "enum", "eval", "export", "extends", "false", "finally",
    "for", "function", "if", "implements", "import", "in", "instanceof", "interface", "let", "new",
    "null", "package", "private", "protected", "public", "return", "static", "super", "switch",
    "this", "throw", "true", "try", "typeof", "undefined", "var", "void", "while", "with", "yield",
    "constructor", "prototype",
];

const JS_GLOBALS: &[&str] = &[
    "Array", "BigInt", "Boolean", "Date", "Error", "Function", "Infinity", "JSON", "Map", "Math",
    "NaN", "Number", "Object", "Promise", "Reflect", "RegExp", "Set", "String", "Symbol", "WeakMap",
    "console", "globalThis", "process", "require", "module",
];

fn op_char_name(c: char) -> Option<&'static str> {
    Some(match c {
        '+' => "$plus",
        '-' => "$minus",
        '*' => "$times",
        '/' => "$div",
        '%' => "$percent",
        '<' => "$less",
        '>' => "$greater",
        '=' => "$eq",
        '!' => "$bang",
        '&' => "$amp",
        '|' => "$bar",
        '^' => "$up",
        '~' => "$tilde",
        ':' => "$colon",
        '?' => "$qmark",
        '#' => "$hash",
        '@' => "$at",
        '\\' => "$bslash",
        _ => return None,
    })
}

/// Turns a Scala identifier into a valid JS identifier.
pub fn sanitize(name: &str) -> String {
    let mut out = String::with_capacity(name.len() + 4);
    for c in name.chars() {
        match op_char_name(c) {
            Some(s) => out.push_str(s),
            None if c.is_alphanumeric() || c == '_' || c == '$' => out.push(c),
            None => {
                out.push_str("$u");
                out.push_str(&(c as u32).to_string());
            }
        }
    }
    if out.is_empty() || out.as_bytes()[0].is_ascii_digit() {
        out.insert(0, '$');
    }
    if RESERVED.contains(&out.as_str()) {
        out.push('$');
    }
    out
}

/// Where a module-level definition goes by its bare name, without the prefix of its package: a
/// name that no other definition of the program has is bare everywhere; one that another
/// package defines as well is bare in the scope of its own package and qualified in the others;
/// one that its own scope holds twice (the `Map`s of the standard library, which is one module)
/// is qualified everywhere. In a single-file build the whole program is one scope.
#[derive(Clone, Copy, PartialEq, Eq, Default)]
pub enum NameForm {
    #[default]
    Bare,
    BareInScope,
    Qualified,
}

/// The scope of a definition that belongs to no package module: the entry point.
pub const NO_SCOPE: u32 = u32::MAX;

pub struct Naming {
    pub classes: Vec<NameForm>,
    pub syms: Vec<NameForm>,
    /// Per file, the scope of its definitions: the standard library, or the package.
    file_scope: Vec<u32>,
    /// The identifiers a bare name must not take: the globals the program refers to and the
    /// helpers of the runtime.
    taken: FxMap<String, ()>,
}

impl Naming {
    pub fn compute(
        prog: &Program,
        syms: &Symbols,
        interner: &Interner,
        reach: &super::reach::Reach,
        const_vals: &[bool],
        split: Option<(&[SourceFile], &[PkgId])>,
        helpers: &[&str],
    ) -> Naming {
        let file_scope: Vec<u32> = match split {
            None => Vec::new(),
            Some((files, pkgs)) => {
                pkgs.iter().enumerate().map(|(i, p)| if files.get(i).map_or(false, |f| f.is_std) { 0 } else { p.0 + 1 }).collect()
            }
        };
        let scope_of = |file: FileId| file_scope.get(file.0 as usize).copied().unwrap_or(0);
        let mut taken: FxMap<String, ()> = FxMap::default();
        for h in helpers {
            taken.insert(h.to_string(), ());
        }
        let mut keyed: Vec<(bool, usize, u64, u32)> = Vec::with_capacity(syms.classes.len() * 2);
        let mut total: FxMap<u64, u32> = FxMap::with_capacity_and_hasher(syms.classes.len() * 2, Default::default());
        for (i, info) in syms.classes.iter().enumerate() {
            match info.js_binding {
                Some(JsBinding::Global(n)) => {
                    taken.insert(interner.get(n).to_string(), ());
                }
                Some(JsBinding::GlobalScope) => {
                    for &m in &info.member_order {
                        taken.insert(interner.get(syms.js_member_name(m)).to_string(), ());
                    }
                }
                _ => {}
            }
            if info.js != JsKind::Scala {
                for &m in &info.member_order {
                    if let Some(g) = syms.sym(m).js_global {
                        taken.insert(interner.get(g).to_string(), ());
                    }
                }
            }
            if let Some(sym) = info.singleton {
                let mut h = class_key_hasher(syms, interner, ClassId(i as u32));
                h.write_u8(4);
                let key = h.finish();
                *total.entry(key).or_insert(0) += 1;
                keyed.push((false, sym.idx(), key, scope_of(info.file)));
            }
            // Only what the output holds competes for a name.
            if info.owner == Owner::Local || info.js_binding.is_some() || !reach.classes.get(i).copied().unwrap_or(true) {
                continue;
            }
            let tag = match info.kind {
                ClassKind::Object => 1,
                _ if super::beside_companion(interner, info) => 1,
                ClassKind::GivenImpl => 2,
                ClassKind::EnumCase if info.singleton.is_some() => 3,
                _ => 0,
            };
            let key = class_key(syms, interner, ClassId(i as u32), tag);
            *total.entry(key).or_insert(0) += 1;
            keyed.push((true, i, key, scope_of(info.module_file())));
        }
        let top_syms = prog.top_funs.iter().map(|&f| prog.funs[f.idx()].sym).chain(prog.top_vals.iter().map(|&(s, _)| s));
        for s in top_syms {
            let info = syms.sym(s);
            if let Some(g) = info.js_global {
                taken.insert(interner.get(g).to_string(), ());
            }
            if !matches!(info.owner, Owner::Package(_)) {
                continue;
            }
            let accessor = !info.is_extension && matches!(info.kind, SymKind::Val | SymKind::Var | SymKind::Given) && !const_vals[s.idx()];
            // A val's accessor is `name$`, which a top-level object's accessor is too
            // (`experimental.RequestRedirect` next to `dom.RequestRedirect`): both are keyed alike.
            let mut h = FxHasher::default();
            let text = interner.get(syms.dispatch_name(s));
            if accessor {
                h.write_u32(text.len() as u32);
                h.write(text.as_bytes());
                h.write_u8(1);
            } else {
                h.write_u8(5);
                h.write_u32(text.len() as u32);
                h.write(text.as_bytes());
            }
            // An extension goes by its name and its rank among its package's of that name; two
            // packages' extensions of one name and rank meet in a module importing both.
            if info.is_extension {
                h.write_u8(2);
                h.write_u32(super::layout::extension_rank(syms, interner, s) as u32);
            } else if !accessor {
                h.write_u8(0);
            }
            let key = h.finish();
            *total.entry(key).or_insert(0) += 1;
            keyed.push((false, s.idx(), key, scope_of(info.file)));
        }
        // The few names the program has twice are counted per scope.
        let mut in_scope: FxMap<u64, u32> = FxMap::default();
        let scoped = |key: u64, scope: u32| key ^ (scope as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15);
        for &(_, _, key, scope) in &keyed {
            if total[&key] > 1 {
                *in_scope.entry(scoped(key, scope)).or_insert(0) += 1;
            }
        }
        let mut naming = Naming {
            classes: vec![NameForm::Bare; syms.classes.len()],
            syms: vec![NameForm::Bare; syms.syms.len()],
            file_scope,
            taken,
        };
        for (is_class, i, key, scope) in keyed {
            let form = if total[&key] == 1 {
                NameForm::Bare
            } else if in_scope[&scoped(key, scope)] == 1 {
                NameForm::BareInScope
            } else {
                NameForm::Qualified
            };
            if is_class {
                naming.classes[i] = form;
            } else {
                naming.syms[i] = form;
            }
        }
        naming
    }

    pub fn scope_of(&self, file: FileId) -> u32 {
        self.file_scope.get(file.0 as usize).copied().unwrap_or(0)
    }

    /// Whether a definition of the given form in the given file is written qualified in `scope`.
    pub fn qualified(&self, form: NameForm, file: FileId, scope: u32) -> bool {
        match form {
            NameForm::Bare => false,
            NameForm::BareInScope => self.scope_of(file) != scope,
            NameForm::Qualified => true,
        }
    }

    /// Keeps a bare name off the globals and helpers the output refers to; `$$` is no suffix of
    /// another kind of definition.
    pub fn avoid_taken(&self, name: &mut String) {
        if JS_GLOBALS.contains(&name.as_str()) || self.taken.contains_key(name.as_str()) {
            name.push_str("$$");
        }
    }
}

/// The key of a class's name in the output: the texts of its name and its owners' (with the
/// length of each, so that two chains of other segments never give one byte stream), the
/// same on every worker whatever order the names were interned in.
fn class_key_hasher(syms: &Symbols, interner: &Interner, c: ClassId) -> FxHasher {
    let mut h = FxHasher::default();
    let mut at = Some(c);
    while let Some(k) = at {
        let info = syms.class(k);
        let text = interner.get(info.name);
        h.write_u32(text.len() as u32);
        h.write(text.as_bytes());
        at = match info.owner {
            Owner::Class(p) => Some(p),
            _ => None,
        };
    }
    h
}

fn class_key(syms: &Symbols, interner: &Interner, c: ClassId, tag: u8) -> u64 {
    let mut h = class_key_hasher(syms, interner, c);
    h.write_u8(tag);
    h.finish()
}

pub fn is_js_identifier(s: &str) -> bool {
    let mut chars = s.chars();
    chars.next().map_or(false, |c| c.is_ascii_alphabetic() || c == '_' || c == '$')
        && chars.all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '$')
}

pub fn js_string(s: &str, out: &mut String) {
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            '\u{2028}' => out.push_str("\\u2028"),
            '\u{2029}' => out.push_str("\\u2029"),
            c if (c as u32) < 0x20 => {
                out.push_str(&format!("\\u{:04x}", c as u32));
            }
            c if crate::text::surrogate_of(c).is_some() => {
                out.push_str(&format!("\\u{:04x}", crate::text::surrogate_of(c).unwrap()));
            }
            c => out.push(c),
        }
    }
    out.push('"');
}

/// The short names the members of Scala classes take under `--release`: one table for the
/// whole program, so that a call by name lands on the same member in every module. A name
/// JavaScript code can see keeps its own: what the runtime calls on any value, what a `js.Dynamic`
/// selection or a `@js` template of the program names, and every member of a class that an
/// exported definition takes or returns, or that such a class takes or returns in turn.
pub struct Renames {
    table: FxMap<String, String>,
}

/// The members the runtime calls on values of any class, and `exception` of a
/// `JavaScriptException`, which it unwraps.
const RUNTIME_CALLED: &[&str] = &[
    "toString", "equals", "hashCode", "foreach", "iterator", "hasNext", "next", "drop", "apply", "applyOrElse",
    "isDefinedAt", "_1", "_2", "constructor", "exception", "productArity", "productElement", "productPrefix",
    "compareTo",
];

/// Property names that the runtime and the emitter put on instances behind a `$`, which a short
/// name must not meet through the `$` of an accessor-backed field.
const RUNTIME_PROPERTIES: &[&str] = &["name", "ordinal", "className", "fields", "own", "body", "traitInit", "init"];

impl Renames {
    pub fn compute(
        syms: &Symbols,
        reach: &super::reach::Reach,
        member_name: impl Fn(SymId) -> String,
        extension_names: impl Fn(ClassId) -> Vec<(SymId, String)>,
        primitive_trait: impl Fn(ClassId) -> bool,
    ) -> Renames {
        let mut kept: FxMap<String, ()> = FxMap::default();
        for n in RUNTIME_CALLED {
            kept.insert(n.to_string(), ());
        }
        for n in &reach.js_names {
            kept.insert(n.clone(), ());
        }
        // A type a JS primitive implements (`CharSequence`, a JS string; `Number`) dispatches its members
        // by their own names: the primitive has them natively (`charAt`) or the std's templates
        // name them (`length`), and a class implementing the trait answers to the same.
        for (i, info) in syms.classes.iter().enumerate() {
            let c = ClassId(i as u32);
            if reach.classes.get(i).copied().unwrap_or(false) && primitive_trait(c) {
                for &m in &info.member_order {
                    kept.insert(member_name(m), ());
                }
            }
        }
        let mut names: Vec<String> = Vec::new();
        for (i, info) in syms.classes.iter().enumerate() {
            let c = ClassId(i as u32);
            if !reach.classes[i] || info.js != JsKind::Scala {
                continue;
            }
            for &m in info.member_order.iter().chain(info.ctor_syms.iter().flatten()) {
                let sym = syms.sym(m);
                if matches!(sym.kind, SymKind::Val | SymKind::Var | SymKind::Def | SymKind::Given) && sym.owner == Owner::Class(c) && !syms.product_synthetics.contains_key(&m) {
                    names.push(member_name(m));
                }
            }
            for (_, n) in extension_names(c) {
                names.push(n);
            }
        }
        names.sort_unstable();
        names.dedup();
        let mut table: FxMap<String, String> = FxMap::default();
        let mut next = 0u32;
        for dev in names {
            if kept.contains_key(&dev) {
                continue;
            }
            let short = loop {
                let candidate = short_name(next);
                next += 1;
                let free = !kept.contains_key(&candidate)
                    && !RESERVED.contains(&candidate.as_str())
                    && !RUNTIME_PROPERTIES.contains(&candidate.as_str());
                if free {
                    break candidate;
                }
            };
            table.insert(dev, short);
        }
        Renames { table }
    }

    pub fn get(&self, dev: &str) -> String {
        self.table.get(dev).cloned().unwrap_or_else(|| dev.to_string())
    }
}

/// `a`..`z`, `A`..`Z`, `aa`, `ab`, ... for `n` = 0, 1, ...
fn short_name(mut n: u32) -> String {
    const LETTERS: &[u8] = b"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ";
    let mut out = Vec::new();
    loop {
        out.push(LETTERS[(n % 52) as usize]);
        n /= 52;
        if n == 0 {
            break;
        }
        n -= 1;
    }
    out.reverse();
    String::from_utf8(out).unwrap()
}
