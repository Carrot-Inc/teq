//! The identifiers of the locals of the output, chosen against capture (docs/TARGETS.md, "The
//! JavaScript output"). JavaScript resolves a bare identifier to the innermost binding in scope,
//! and a `const` or `let` binds through its whole block, so a local's name must not be one that
//! the block's text reads for something else: a definition of the program, an import, a
//! global, a helper of the runtime, a local of an enclosing scope, or a name that a binder
//! nested in the block gives a read of the local itself.

use super::expr::{double_global, prim_text, test_text, to_str_text, unary_text, THROW, UNWRAP_JS};
use super::names::{is_js_identifier, sanitize, RESERVED};
use super::share::{HoleKind, HoleNode};
use super::Emitter;
use crate::ast::{mods, ListRef};
use crate::intern::{FxMap, Interner};
use crate::symbols::*;
use crate::tir::*;
use crate::types::{ClassId, SymId};
use std::rc::Rc;

/// What a text the output writes inside a body binds and reads: a `@js` template, or the fixed
/// text of one of the emitter's operations.
#[derive(Default, Debug, PartialEq)]
pub(super) struct TextNames {
    /// The identifiers the text reads and does not declare.
    pub free: Vec<String>,
    /// Per placeholder `$n` of the text, the names the text declares around it.
    pub around: Vec<(u8, Vec<String>)>,
}

#[derive(Clone, Copy, PartialEq, Debug)]
enum Tok {
    Ident,
    Hole(u8),
    Open(u8),
    Close(u8),
    Arrow,
    Dot,
    Spread,
    Punct(u8),
    Value,
}

/// The text of a template literal from `i` on: the position after its closing backtick, or after
/// the `${` that opens an interpolation, which the second says.
fn template_text(b: &[u8], mut i: usize) -> (usize, bool) {
    while i < b.len() {
        match b[i] {
            b'\\' => i += 2,
            b'`' => return (i + 1, false),
            b'$' if b.get(i + 1) == Some(&b'{') => return (i + 2, true),
            _ => i += 1,
        }
    }
    (b.len(), false)
}

fn lex(t: &str) -> Vec<(Tok, &str)> {
    let b = t.as_bytes();
    let mut out: Vec<(Tok, &str)> = Vec::new();
    // Per interpolation of a template literal being read, the braces open inside it: its code is
    // read as a parenthesised expression.
    let mut interpolations: Vec<u32> = Vec::new();
    let mut i = 0;
    while i < b.len() {
        let c = b[i];
        let start = i;
        if c.is_ascii_whitespace() {
            i += 1;
            continue;
        }
        if c == b'$' && b.get(i + 1).map_or(false, |d| d.is_ascii_digit()) {
            out.push((Tok::Hole(b[i + 1] - b'0'), &t[i..i + 2]));
            i += 2;
            continue;
        }
        let ident_char = |c: u8| c.is_ascii_alphanumeric() || c == b'_' || c == b'$' || c >= 0x80;
        if ident_char(c) && !c.is_ascii_digit() {
            while i < b.len() && ident_char(b[i]) {
                i += 1;
            }
            out.push((Tok::Ident, &t[start..i]));
            continue;
        }
        if c.is_ascii_digit() {
            while i < b.len() && (ident_char(b[i]) || b[i] == b'.') {
                i += 1;
            }
            out.push((Tok::Value, &t[start..i]));
            continue;
        }
        let next = b.get(i + 1).copied().unwrap_or(0);
        let tok = match c {
            b'`' => {
                let (end, interpolation) = template_text(b, i + 1);
                i = end;
                if !interpolation {
                    out.push((Tok::Value, &t[start..end]));
                    continue;
                }
                interpolations.push(0);
                Tok::Open(b'(')
            }
            b'}' if interpolations.last() == Some(&0) => {
                interpolations.pop();
                out.push((Tok::Close(b')'), &t[i..i + 1]));
                let (end, interpolation) = template_text(b, i + 1);
                i = end;
                if interpolation {
                    interpolations.push(0);
                    out.push((Tok::Open(b'('), &t[end - 2..end]));
                }
                continue;
            }
            b'"' | b'\'' => {
                i += 1;
                while i < b.len() && b[i] != c {
                    if b[i] == b'\\' {
                        i += 1;
                    }
                    i += 1;
                }
                i += 1;
                Tok::Value
            }
            b'/' if next == b'/' => {
                while i < b.len() && b[i] != b'\n' {
                    i += 1;
                }
                continue;
            }
            b'/' if next == b'*' => {
                i += 2;
                while i + 1 < b.len() && !(b[i] == b'*' && b[i + 1] == b'/') {
                    i += 1;
                }
                i += 2;
                continue;
            }
            b'/' if regex_may_follow(out.last()) => {
                i += 1;
                let mut class = false;
                while i < b.len() && (class || b[i] != b'/') {
                    match b[i] {
                        b'\\' => i += 1,
                        b'[' => class = true,
                        b']' => class = false,
                        _ => {}
                    }
                    i += 1;
                }
                i += 1;
                while i < b.len() && b[i].is_ascii_alphabetic() {
                    i += 1;
                }
                Tok::Value
            }
            b'(' | b'[' | b'{' => {
                if let (b'{', Some(open)) = (c, interpolations.last_mut()) {
                    *open += 1;
                }
                i += 1;
                Tok::Open(c)
            }
            b')' | b']' | b'}' => {
                if let (b'}', Some(open)) = (c, interpolations.last_mut()) {
                    *open -= 1;
                }
                i += 1;
                Tok::Close(c)
            }
            b'=' if next == b'>' => {
                i += 2;
                Tok::Arrow
            }
            b'.' if next == b'.' && b.get(i + 2) == Some(&b'.') => {
                i += 3;
                Tok::Spread
            }
            b'.' => {
                i += 1;
                Tok::Dot
            }
            b'?' if next == b'.' && !b.get(i + 2).map_or(false, |d| d.is_ascii_digit()) => {
                i += 2;
                Tok::Dot
            }
            _ => {
                i += 1;
                Tok::Punct(c)
            }
        };
        out.push((tok, &t[start..i.min(b.len())]));
    }
    out
}

/// Whether a `/` after the token starts a regular expression rather than a division.
fn regex_may_follow(prev: Option<&(Tok, &str)>) -> bool {
    match prev {
        None => true,
        Some((Tok::Ident, s)) => matches!(*s, "return" | "typeof" | "case" | "do" | "else" | "in" | "of" | "new" | "delete" | "void" | "throw" | "instanceof" | "yield" | "await"),
        Some((Tok::Hole(_) | Tok::Value | Tok::Close(_), _)) => false,
        Some(_) => true,
    }
}

/// The free identifiers of a text the output writes and the names it declares around each of
/// its placeholders: a `const` or `let` binds through its enclosing braces, a parameter of an
/// arrow function, a `function` or a `catch` through its body.
pub(super) fn text_names(text: &str) -> TextNames {
    let toks = lex(text);
    let n = toks.len();
    let mut partner = vec![usize::MAX; n];
    let mut parent = vec![usize::MAX; n];
    let mut stack: Vec<usize> = Vec::new();
    for i in 0..n {
        parent[i] = stack.last().copied().unwrap_or(usize::MAX);
        match toks[i].0 {
            Tok::Open(_) => stack.push(i),
            Tok::Close(_) => {
                if let Some(o) = stack.pop() {
                    partner[o] = i;
                    partner[i] = o;
                    parent[i] = parent[o];
                }
            }
            _ => {}
        }
    }
    // `of` is a keyword after the binding of a `for` head; `get`, `set`, `async` and the other
    // contextual words are identifiers where they bind or are read.
    let for_of = |i: usize| {
        toks[i].1 == "of" && parent[i] != usize::MAX && parent[i] > 0 && toks[parent[i]].0 == Tok::Open(b'(') && toks[parent[i] - 1].1 == "for"
    };
    let ident = |i: usize| i < n && toks[i].0 == Tok::Ident && !RESERVED.contains(&toks[i].1) && !for_of(i);
    // The body of the function a token stands in: an arrow's or a `function`'s braces.
    let function_around = |i: usize| -> (usize, usize) {
        let mut at = parent[i];
        while at != usize::MAX {
            if toks[at].0 == Tok::Open(b'{') && partner[at] != usize::MAX && at > 0 {
                let body = match toks[at - 1].0 {
                    Tok::Arrow => true,
                    Tok::Close(b')') if partner[at - 1] != usize::MAX => {
                        let open = partner[at - 1];
                        (open >= 1 && toks[open - 1].1 == "function") || (open >= 2 && toks[open - 1].0 == Tok::Ident && toks[open - 2].1 == "function")
                    }
                    _ => false,
                };
                if body {
                    return (at, partner[at]);
                }
            }
            at = parent[at];
        }
        (0, n.saturating_sub(1))
    };
    let braces_around = |i: usize| -> (usize, usize) {
        let mut at = parent[i];
        while at != usize::MAX {
            if toks[at].0 == Tok::Open(b'{') && partner[at] != usize::MAX {
                return (at, partner[at]);
            }
            at = parent[at];
        }
        (0, n.saturating_sub(1))
    };
    // The names a binding pattern from `open` to its close binds: a parameter list `( ... )` or a
    // destructuring `[ ... ]`, `{ ... }`; the keys of an object pattern and what defaults read
    // are no bindings.
    let bound_in = |open: usize, out: &mut Vec<usize>| {
        let close = partner[open];
        if close == usize::MAX {
            return;
        }
        for j in open + 1..close {
            let starts = matches!(toks[j - 1].0, Tok::Open(b'(' | b'[' | b'{') | Tok::Punct(b',' | b':') | Tok::Spread);
            let key = j + 1 < n && toks[j + 1].0 == Tok::Punct(b':');
            // Inside the pattern's own brackets, not inside a default's call.
            let mut at = parent[j];
            while at != open && at != usize::MAX && matches!(toks[at].0, Tok::Open(b'[' | b'{')) {
                at = parent[at];
            }
            if ident(j) && starts && !key && at == open {
                out.push(j);
            }
        }
    };
    // (name token, first token of the extent, last token of the extent)
    let mut binders: Vec<(usize, usize, usize)> = Vec::new();
    for i in 0..n {
        match (toks[i].0, toks[i].1) {
            (Tok::Ident, kind @ ("const" | "let" | "var")) => {
                // A `var` binds through its whole function, before its declaration as well.
                let (from, to) = if kind == "var" { function_around(i) } else { braces_around(i) };
                let mut j = i + 1;
                let mut depth = 0i32;
                let mut expect = true;
                while j < n {
                    match toks[j].0 {
                        Tok::Open(b'[' | b'{') if depth == 0 && expect => {
                            let mut names = Vec::new();
                            bound_in(j, &mut names);
                            binders.extend(names.into_iter().map(|at| (at, from, to)));
                            expect = false;
                            if partner[j] == usize::MAX {
                                break;
                            }
                            j = partner[j];
                        }
                        Tok::Open(_) => depth += 1,
                        Tok::Close(_) if depth == 0 => break,
                        Tok::Close(_) => depth -= 1,
                        Tok::Punct(b';') if depth == 0 => break,
                        Tok::Punct(b',') if depth == 0 => expect = true,
                        Tok::Ident if depth == 0 && (toks[j].1 == "in" || for_of(j)) => break,
                        Tok::Ident if depth == 0 && expect && ident(j) => {
                            binders.push((j, from, to));
                            expect = false;
                        }
                        _ => expect = false,
                    }
                    j += 1;
                }
            }
            (Tok::Ident, "function") => {
                let mut j = i + 1;
                let name = ident(j).then_some(j);
                if name.is_some() {
                    j += 1;
                }
                if j < n && toks[j].0 == Tok::Open(b'(') && partner[j] != usize::MAX {
                    let body = partner[j] + 1;
                    let end = if body < n && toks[body].0 == Tok::Open(b'{') { partner[body] } else { partner[j] };
                    // A declaration's name binds through its block, an expression's in its own body.
                    if let Some(at) = name {
                        let declaration = i == 0 || matches!(toks[i - 1].0, Tok::Open(b'{') | Tok::Close(b'}') | Tok::Punct(b';'));
                        let (from, to) = if declaration { braces_around(i) } else { (i, end) };
                        binders.push((at, from, to));
                    }
                    let mut ps = Vec::new();
                    bound_in(j, &mut ps);
                    binders.extend(ps.into_iter().map(|p| (p, j, end)));
                }
            }
            (Tok::Ident, "catch") if i + 1 < n && toks[i + 1].0 == Tok::Open(b'(') && partner[i + 1] != usize::MAX => {
                let body = partner[i + 1] + 1;
                let end = if body < n && toks[body].0 == Tok::Open(b'{') { partner[body] } else { partner[i + 1] };
                let mut ps = Vec::new();
                bound_in(i + 1, &mut ps);
                binders.extend(ps.into_iter().map(|p| (p, i + 1, end)));
            }
            (Tok::Arrow, _) if i > 0 => {
                let (from, mut ps) = match toks[i - 1].0 {
                    Tok::Ident if ident(i - 1) => (i - 1, vec![i - 1]),
                    Tok::Close(b')') if partner[i - 1] != usize::MAX => {
                        let mut ps = Vec::new();
                        bound_in(partner[i - 1], &mut ps);
                        (partner[i - 1], ps)
                    }
                    _ => continue,
                };
                let end = if i + 1 < n && toks[i + 1].0 == Tok::Open(b'{') && partner[i + 1] != usize::MAX {
                    partner[i + 1]
                } else {
                    let mut k = i + 1;
                    let mut depth = 0i32;
                    while k < n {
                        match toks[k].0 {
                            Tok::Open(_) => depth += 1,
                            Tok::Close(_) if depth == 0 => break,
                            Tok::Close(_) => depth -= 1,
                            Tok::Punct(b',' | b';') if depth == 0 => break,
                            _ => {}
                        }
                        k += 1;
                    }
                    k.saturating_sub(1)
                };
                binders.extend(ps.drain(..).map(|p| (p, from, end)));
            }
            _ => {}
        }
    }
    let declared = |i: usize| binders.iter().any(|&(at, _, _)| at == i);
    let bound = |name: &str, i: usize| binders.iter().any(|&(at, from, to)| toks[at].1 == name && from <= i && i <= to);
    let mut names = TextNames::default();
    for i in 0..n {
        match toks[i].0 {
            Tok::Ident if ident(i) && !declared(i) => {
                let name = toks[i].1;
                let member = i > 0 && toks[i - 1].0 == Tok::Dot;
                let key = i + 1 < n
                    && toks[i + 1].0 == Tok::Punct(b':')
                    && i > 0
                    && matches!(toks[i - 1].0, Tok::Open(b'{') | Tok::Punct(b','))
                    && parent[i] != usize::MAX
                    && toks[parent[i]].0 == Tok::Open(b'{');
                if !member && !key && !bound(name, i) && !names.free.iter().any(|f| f == name) {
                    names.free.push(name.to_string());
                }
            }
            Tok::Hole(k) => {
                let mut around: Vec<String> = Vec::new();
                for &(at, from, to) in &binders {
                    if from <= i && i <= to && !around.iter().any(|a| a == toks[at].1) {
                        around.push(toks[at].1.to_string());
                    }
                }
                match names.around.iter_mut().find(|(h, _)| *h == k) {
                    Some((_, prev)) => {
                        for a in around {
                            if !prev.contains(&a) {
                                prev.push(a);
                            }
                        }
                    }
                    None => names.around.push((k, around)),
                }
            }
            _ => {}
        }
    }
    names
}

/// The identifier a text of a binding starts with: `$imp3` of `$imp3.default`.
pub(super) fn identifier_of(text: &str) -> &str {
    let end = text.bytes().position(|c| !(c.is_ascii_alphanumeric() || c == b'_' || c == b'$' || c >= 0x80)).unwrap_or(text.len());
    &text[..end]
}

/// The part of an identifier a capture can be told by without the whole set of names the output
/// renders: the text up to the first `$` after its first character. Every form the emitter writes
/// for one definition (`Foo$M`, its accessor `Foo$` and instance `Foo$i`; `x$`, `x$v`, `x$set`)
/// adds to its name after a `$`, so the forms share the stem of the name.
pub(super) fn stem(id: &str) -> &str {
    match id.as_bytes().iter().skip(1).position(|&c| c == b'$') {
        Some(p) => &id[..p + 1],
        None => id,
    }
}

/// What the output can render as a bare identifier anywhere, as stems, and what each `@js`
/// template of the program binds and reads. The stems only spare the emitter the walk of a
/// scope when no name of the kind the local takes is rendered anywhere: whether a local is
/// renamed depends on the scope's own text alone, so that the names do not depend on what
/// else the build holds.
#[derive(Default)]
pub struct Renderable<'a> {
    stems: FxMap<String, ()>,
    /// By text: a program holds a few hundred templates, each written in many places.
    templates: FxMap<&'a str, TextNames>,
    /// By class, the name the output writes for each class it names, before its package's prefix
    /// (`bare_class_text`): `Emitter::class_text` writes it, and the stems are taken from it and
    /// from the accessor and instance an object's name gives (`accessor_of`, `instance_of`).
    class_names: Vec<Option<Box<str>>>,
}

impl<'a> Renderable<'a> {
    #[allow(clippy::too_many_arguments)]
    pub(super) fn compute(
        prog: &'a Program,
        syms: &Symbols,
        interner: &Interner,
        reach: &super::reach::Reach,
        naming: &super::names::Naming,
        shared: &super::share::Shared,
        const_vals: &[bool],
        file_inits: &[bool],
        helpers: &[&str],
        written: impl Iterator<Item = &'static str>,
        outline_names: impl Iterator<Item = String>,
        shared_names: impl Iterator<Item = String>,
    ) -> Renderable<'a> {
        let mut r = Renderable::default();
        let add = |stems: &mut FxMap<String, ()>, id: &str| {
            let s = stem(id);
            if !stems.contains_key(s) {
                stems.insert(s.to_string(), ());
            }
        };

        // The fixed binders of the emitter around reads of locals (`$ref`'s setter, a predicate).
        for id in ["$v", "$", "globalThis", "Symbol"] {
            add(&mut r.stems, id);
        }
        for id in helpers {
            add(&mut r.stems, id);
        }
        for t in written {
            for id in text_names(t).free {
                add(&mut r.stems, &id);
            }
        }
        for id in outline_names.chain(shared_names) {
            add(&mut r.stems, &id);
        }
        for i in 0..prog.js_imports.len() {
            add(&mut r.stems, &format!("$imp{}", i));
        }
        for (f, _) in file_inits.iter().enumerate().filter(|(_, &has)| has) {
            add(&mut r.stems, &prog.file_init_name(crate::source::FileId(f as u32)));
        }
        for p in syms.pkgs.iter() {
            if p.parent.map_or(false, |q| syms.pkg(q).parent.is_none()) {
                add(&mut r.stems, &sanitize(interner.get(p.name)));
            }
        }
        // A reference to a definition bound to a global is the typer's `JsGlobal`, among
        // `Reach::globals`. The names of the program's definitions are the ones the emitter
        // writes (`bare_package_sym_text`, `bare_class_text`), before their packages' prefixes.
        let funs = prog.top_funs.iter().filter(|&&f| reach.funs[f.idx()]).map(|&f| prog.funs[f.idx()].sym);
        for s in funs.chain(prog.top_vals.iter().map(|&(s, _)| s)) {
            add(&mut r.stems, &super::bare_package_sym_text(syms, interner, naming, const_vals, s));
        }
        r.class_names = vec![None; syms.classes.len()];
        for (i, info) in syms.classes.iter().enumerate() {
            // What the output names: a reached class, the accessor of an object, a native binding.
            if !reach.classes.get(i).copied().unwrap_or(false) && info.kind != ClassKind::Object && info.js_binding.is_none() {
                continue;
            }
            match info.js_binding {
                None => {
                    let c = shared.rep_of(ClassId(i as u32));
                    if syms.class(c).js_binding.is_some() {
                        continue;
                    }
                    if r.class_names[c.idx()].is_some() {
                        continue;
                    }
                    let name = super::bare_class_text(prog, syms, interner, reach, naming, shared, c);
                    add(&mut r.stems, &name);
                    if super::is_module(syms, interner, c) {
                        let accessor = super::accessor_of(&name);
                        add(&mut r.stems, &super::instance_of(&accessor));
                        add(&mut r.stems, &accessor);
                    }
                    r.class_names[c.idx()] = Some(name.into_boxed_str());
                }
                Some(JsBinding::Global(n)) => add(&mut r.stems, interner.get(n)),
                Some(JsBinding::GlobalScope) => {
                    for &m in &info.member_order {
                        add(&mut r.stems, interner.get(syms.js_member_name(m)));
                    }
                }
                Some(JsBinding::Import(_)) => {}
            }
        }
        for &t in &reach.templates {
            let text = prog.strings[t.idx()].as_str();
            if r.templates.contains_key(text) {
                continue;
            }
            let names = text_names(text);
            for id in names.free.iter().chain(names.around.iter().flat_map(|(_, a)| a.iter())) {
                add(&mut r.stems, id);
            }
            r.templates.insert(text, names);
        }
        for &name in &reach.globals {
            add(&mut r.stems, interner.get(name));
        }
        r
    }

    /// Whether a local named `id` may meet a name the output renders: false only where none of
    /// the stem of `id` is rendered anywhere.
    pub(super) fn may_meet(&self, id: &str) -> bool {
        self.stems.contains_key(stem(id))
    }

    pub(super) fn template(&self, text: &str) -> Option<&TextNames> {
        self.templates.get(text)
    }

    /// The name of the class `c` (a shared group's representative), where the output names it.
    pub(super) fn class_name(&self, c: ClassId) -> Option<&str> {
        self.class_names.get(c.idx()).and_then(|n| n.as_deref())
    }
}

/// What a scope of the output is written from: the key its walk is kept under for the item.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub(super) enum Root {
    /// A block, or the body of a function made where an expression stands, written from it.
    Expr(TExprId),
    /// A def: its parameters, their defaults and its body.
    Fun(FunId),
    /// A lambda: its parameters and its body.
    Lambda(TExprId),
    /// The arrow function of the anonymous class made by the `New` at this expression.
    Closure(TExprId),
    /// The cases from the first index of `Program::cases` on, as many as the second says: the
    /// block of a match's or a `try`'s cases, or of the cases that share a type test.
    Cases(u32, u32),
    /// The case at this index of `Program::cases`: its pattern's bindings, guard and body.
    Case(u32),
    /// The constructor of the class at this index of `Program::classes`, and its `$body`.
    Ctor(u32),
    /// The getter of a lazy field: its initialiser.
    Getter(TExprId),
    /// The initialiser of the vals of a file (`Emitter::group_inits`).
    Vals,
}

/// A block or a function of the output being written.
pub(super) struct Scope {
    pub root: Root,
    /// `names` as a set.
    pub declared: FxMap<Rc<str>, ()>,
    /// Unique among the scopes an emitter opens, so that a declaration registered for a scope
    /// does not find another one later at its depth.
    pub id: u32,
    /// The names declared directly in it.
    pub names: Vec<Rc<str>>,
    /// Those of them that are no locals of the source, the emitter's own temporaries and
    /// parameters, whose reads the walk of a root does not see.
    pub fixed: Vec<Rc<str>>,
}

/// A binder around a read of a local: another local, or a name the emitter's own text binds.
#[derive(Clone, PartialEq, Debug)]
pub(super) enum Binder {
    Local(SymId),
    Fixed(Rc<str>),
}

/// What the text of a root reads, as the walk of its tree finds it.
#[derive(Default)]
pub(super) struct RootInfo {
    /// The names it reads that are no locals of the item: definitions of the program, imports,
    /// globals, helpers, and the names the emitter's text binds around it.
    nonlocal: FxMap<Rc<str>, ()>,
    /// The binders inside the root, each with the binder around it (`NONE` at the root's level),
    /// in the order the walk met them: the binders inside one are the ones after it, up to `end`.
    binders: Vec<(Binder, u32)>,
    end: Vec<u32>,
    /// The binder of each local the root declares, and the binders of each fixed name.
    node_of: FxMap<SymId, u32>,
    fixed_nodes: FxMap<Rc<str>, Vec<u32>>,
    /// Per local it reads, the innermost binder around each of its reads, an index of `binders`.
    reads: FxMap<SymId, Vec<u32>>,
    /// Per object whose instance it reads through the accessor, the innermost binder around each
    /// read: what a binding of the instance (`Emitter::close_frame`) stands for.
    modules: FxMap<ClassId, Vec<u32>>,
    /// The locals it declares.
    declared: FxMap<SymId, ()>,
}

impl RootInfo {
    /// Whether the binder `node` stands around a read of `s`.
    fn around(&self, node: u32, s: SymId) -> bool {
        self.reads.get(&s).map_or(false, |tops| self.encloses(node, tops))
    }

    /// Whether the binder `node` stands around a read of the instance of the object `c`.
    fn around_module(&self, node: u32, c: ClassId) -> bool {
        self.modules.get(&c).map_or(false, |tops| self.encloses(node, tops))
    }

    fn encloses(&self, node: u32, tops: &[u32]) -> bool {
        let end = self.end[node as usize];
        tops.iter().any(|&t| t != NONE && node <= t && t < end)
    }
}

const NONE: u32 = u32::MAX;

struct Walk {
    info: RootInfo,
    /// The innermost binder around the node being walked.
    top: u32,
    /// What a lowered anonymous class being walked passes for each local its body captures.
    closure: Vec<(SymId, TExprId)>,
}

impl Walk {
    fn name(&mut self, name: &str) {
        if !self.info.nonlocal.contains_key(name) {
            self.info.nonlocal.insert(Rc::from(name), ());
        }
    }

    fn this(&mut self, this: &str) {
        if this != "this" {
            self.name(this);
        }
    }

    fn read(&mut self, s: SymId) {
        let top = self.top;
        let around = self.info.reads.entry(s).or_default();
        if !around.contains(&top) {
            around.push(top);
        }
    }

    /// A read of the instance of the object `c` through its accessor, named `accessor`.
    fn module(&mut self, c: ClassId, accessor: &str) {
        self.name(accessor);
        let top = self.top;
        let around = self.info.modules.entry(c).or_default();
        if !around.contains(&top) {
            around.push(top);
        }
    }

    fn push(&mut self, b: Binder) {
        self.info.binders.push((b, self.top));
        self.top = (self.info.binders.len() - 1) as u32;
    }

    fn pop(&mut self) {
        self.top = self.info.binders[self.top as usize].1;
    }

    fn bind(&mut self, s: SymId) {
        self.info.declared.insert(s, ());
        self.push(Binder::Local(s));
        self.info.node_of.insert(s, self.top);
    }

    fn bind_fixed(&mut self, name: &str) {
        let name: Rc<str> = Rc::from(name);
        self.push(Binder::Fixed(name.clone()));
        self.info.fixed_nodes.entry(name).or_default().push(self.top);
    }
}

impl<'a> Emitter<'a> {
    pub(super) fn enter_scope(&mut self, root: Root) {
        self.scope_ids += 1;
        let mut scope = Scope { root, declared: FxMap::default(), id: self.scope_ids, names: Vec::new(), fixed: Vec::new() };
        if let Some((names, fixed)) = self.pending_names.get(&root) {
            for n in names {
                *self.open_names.entry(n.clone()).or_insert(0) += 1;
                scope.declared.insert(n.clone(), ());
            }
            scope.names = names.clone();
            scope.fixed = fixed.clone();
        }
        self.scopes.push(scope);
    }

    pub(super) fn leave_scope(&mut self) {
        let scope = self.scopes.pop().expect("a scope is open");
        for n in scope.names {
            if let Some(count) = self.open_names.get_mut(&n) {
                *count -= 1;
                if *count == 0 {
                    self.open_names.remove(&n);
                }
            }
        }
    }

    /// Opens the scope that the bindings of a case are declared in before the block that holds
    /// them is written, and keeps what they were named when it closes, for that block.
    pub(super) fn enter_pending(&mut self, root: Root) {
        self.enter_scope(root);
    }

    pub(super) fn leave_pending(&mut self) {
        let scope = self.scopes.last().expect("a scope is open");
        let (root, names, fixed) = (scope.root, scope.names.clone(), scope.fixed.clone());
        self.leave_scope();
        self.pending_names.insert(root, (names, fixed));
    }

    fn declare_in(&mut self, at: usize, name: Rc<str>) {
        *self.open_names.entry(name.clone()).or_insert(0) += 1;
        self.scopes[at].declared.insert(name.clone(), ());
        self.scopes[at].names.push(name);
    }

    /// Notes that the local `s` goes by `name` from here on: the checks find the locals of a name
    /// through this index, and `close_frame` names an object's binding apart from them.
    pub(super) fn note_bound(&mut self, s: SymId, name: Rc<str>) {
        let named = self.named_as.entry(name).or_default();
        if !named.contains(&s) {
            named.push(s);
        }
    }

    pub(super) fn declare_fixed_in(&mut self, at: usize, name: Rc<str>) {
        self.scopes[at].fixed.push(name.clone());
        self.declare_in(at, name);
    }

    /// A name the emitter's own text declares in the scope being written.
    pub(super) fn declare_fixed(&mut self, name: &str) {
        if let Some(at) = self.scopes.len().checked_sub(1) {
            self.declare_fixed_in(at, Rc::from(name));
        }
    }

    /// The statements of a block are about to be written in the current scope: a local they
    /// declare is named there even where a local def written first reads it.
    pub(super) fn register_decls(&mut self, stmts: &[TStmt]) {
        let Some(scope) = self.scopes.last().map(|s| s.id) else { return };
        let mut binders = Vec::new();
        for s in stmts {
            match *s {
                TStmt::Val(sym, _) => binders.push(sym),
                TStmt::Fun(f) => binders.push(self.prog.funs[f.idx()].sym),
                TStmt::Pat(p, _) => self.pattern_binders(p, &mut binders),
                TStmt::Expr(_) => {}
            }
        }
        for b in binders {
            self.decl_scope.insert(b, scope);
        }
    }

    fn pattern_binders(&self, pat: TPatId, out: &mut Vec<SymId>) {
        let prog = self.prog;
        match prog.pats[pat.idx()] {
            TPat::Wildcard | TPat::Equals(..) => {}
            TPat::Bind(sym, inner) => {
                out.push(sym);
                if let Some(i) = inner {
                    self.pattern_binders(i, out);
                }
            }
            TPat::Unapply(sym, _, inner) => {
                out.push(sym);
                self.pattern_binders(inner, out);
            }
            TPat::Test(_, _, inner) => self.pattern_binders(inner, out),
            TPat::Class(_, _, _, subs) | TPat::Alt(subs) => {
                for &s in &prog.pat_lists[subs.range()] {
                    self.pattern_binders(s, out);
                }
            }
            TPat::Seq(items, rest) => {
                for &s in prog.pat_lists[items.range()].iter().chain(rest.iter()) {
                    self.pattern_binders(s, out);
                }
            }
        }
    }

    /// The open scope a local is declared in.
    fn scope_of_decl(&self, s: SymId) -> Option<usize> {
        if let Some(&id) = self.decl_scope.get(&s) {
            if let Some(at) = self.scopes.iter().rposition(|sc| sc.id == id) {
                return Some(at);
            }
        }
        self.scopes.len().checked_sub(1)
    }

    /// The name of a local that the output declares under another name, as `sym_name` gives
    /// it without the scope it would be declared in.
    pub(super) fn unbound_name(&mut self, s: SymId) -> Rc<str> {
        let open = std::mem::replace(&mut self.unbound, true);
        let name = self.sym_name(s);
        self.unbound = open;
        name
    }

    /// The identifier of the local `s`: apart from a local of an enclosing function or an earlier
    /// one of its source name, and then from whatever would capture it or be captured by it
    /// where it is declared.
    pub(super) fn name_local(&mut self, s: SymId) -> String {
        let info = self.syms.sym(s);
        let source = sanitize(self.interner.get(info.name));
        let mut name = if info.kind != SymKind::Param && self.scope_names.contains(&info.name) {
            self.tmp_counter += 1;
            format!("{}${}", source, self.tmp_counter)
        } else {
            sanitize(self.interner.get(self.syms.dispatch_name(s)))
        };
        let lazy = info.mods & mods::LAZY != 0;
        debug_assert!(!self.walking, "the walk of a scope names the local `{}`", name);
        let Some(at) = self.scope_of_decl(s).filter(|_| !self.unbound) else { return name };
        while self.captured(&name, Some(s), at) || (lazy && (self.captured(&format!("{}$v", name), Some(s), at) || self.captured(&format!("{}$d", name), Some(s), at))) {
            self.tmp_counter += 1;
            name = format!("{}${}", source, self.tmp_counter);
        }
        if lazy {
            self.declare_in(at, Rc::from(format!("{}$v", name)));
            self.declare_in(at, Rc::from(format!("{}$d", name)));
        }
        self.declare_in(at, Rc::from(name.as_str()));
        name
    }

    /// The JavaScript parameter of the constructor's parameter `p`, whose field is named `field`:
    /// that name, unless the constructor reads a global or another binding of it.
    pub(super) fn name_ctor_param(&mut self, p: SymId, field: Rc<str>) -> Rc<str> {
        let mut name = field.to_string();
        if let Some(at) = self.scopes.len().checked_sub(1) {
            while self.captured(&name, Some(p), at) {
                self.tmp_counter += 1;
                name = format!("{}${}", field, self.tmp_counter);
            }
            self.declare_in(at, Rc::from(name.as_str()));
        }
        let name: Rc<str> = Rc::from(name);
        self.note_bound(p, name.clone());
        if *name == *field {
            return field;
        }
        self.ctor_bindings.insert(p, name.clone());
        name
    }

    /// What the constructor being written reads the parameter `p` as.
    pub(super) fn ctor_param(&mut self, p: SymId) -> Rc<str> {
        match self.ctor_bindings.get(&p) {
            Some(n) => n.clone(),
            None => self.sym_name(p),
        }
    }

    fn bound_name(&self, t: SymId) -> Option<&str> {
        match self.ctor_bindings.get(&t) {
            Some(n) => Some(n),
            None => self.sym_names[t.idx()].as_deref(),
        }
    }

    /// A temporary of the emitter, `$<prefix><n>`, declared in the current scope.
    pub(super) fn fresh(&mut self, prefix: &str) -> String {
        let at = self.scopes.len().checked_sub(1);
        self.fresh_in(prefix, at)
    }

    /// A temporary declared in the scope at `at`, which is read where the code being written
    /// stands: apart from the names of the scopes in between as well.
    pub(super) fn fresh_in(&mut self, prefix: &str, at: Option<usize>) -> String {
        loop {
            self.tmp_counter += 1;
            let name = format!("${}{}", prefix, self.tmp_counter);
            let Some(at) = at else { return name };
            let between = self.scopes[at + 1..].iter().any(|sc| sc.declared.contains_key(name.as_str()));
            if !between && !self.captured(&name, None, at) {
                self.declare_fixed_in(at, Rc::from(name.as_str()));
                return name;
            }
        }
    }

    /// A label: labels are no bindings.
    pub(super) fn fresh_label(&mut self, prefix: &str) -> String {
        self.tmp_counter += 1;
        format!("${}{}", prefix, self.tmp_counter)
    }

    /// The parameter of a def written as a loop, `<name>$<n>`, declared in the def's scope.
    pub(super) fn tail_param_name(&mut self, p: SymId) -> Rc<str> {
        let base = sanitize(self.interner.get(self.syms.sym(p).name));
        let at = self.scopes.len().checked_sub(1);
        loop {
            self.tmp_counter += 1;
            let name = format!("{}${}", base, self.tmp_counter);
            let Some(at) = at else { return Rc::from(name) };
            // The loop's jumps, the parameter's only reads, are in the walk of every root inside it.
            if !self.captured(&name, None, at) {
                let name: Rc<str> = Rc::from(name);
                self.declare_in(at, name.clone());
                return name;
            }
        }
    }

    /// The copy of a parameter in each iteration of a def written as a loop: its source name,
    /// unless that is captured in the loop's block.
    pub(super) fn tail_copy_name(&mut self, p: SymId) -> Rc<str> {
        let source = sanitize(self.interner.get(self.syms.sym(p).name));
        let mut name = source.clone();
        if let Some(at) = self.scopes.len().checked_sub(1) {
            while self.captured(&name, Some(p), at) {
                self.tmp_counter += 1;
                name = format!("{}${}", source, self.tmp_counter);
            }
            self.declare_in(at, Rc::from(name.as_str()));
        }
        let name: Rc<str> = Rc::from(name);
        self.note_bound(p, name.clone());
        name
    }

    /// Whether the local `s` (none for a temporary of the emitter) named `name` in the scope at
    /// `at` would read or be read for something else: a name its scope reads that is no local,
    /// the name of a local of an enclosing scope that its scope reads, a binder of its scope
    /// around a read of it, or a name declared in its scope already.
    fn captured(&mut self, name: &str, s: Option<SymId>, at: usize) -> bool {
        if self.assign_targets.iter().any(|t| &**t == name) {
            return true;
        }
        if !self.renderable.may_meet(name) && !self.open_names.contains_key(name) {
            debug_assert!(!self.captured_in_scope(name, s, at), "`{}` is captured where no stem says it may be", name);
            return false;
        }
        self.captured_in_scope(name, s, at)
    }

    /// The check of `captured`, in time of the locals named `name` rather than of the scope's
    /// size.
    fn captured_in_scope(&mut self, name: &str, s: Option<SymId>, at: usize) -> bool {
        if self.open_names.contains_key(name) {
            if self.scopes[at].declared.contains_key(name) {
                return true;
            }
            // A temporary or parameter of the emitter declared around may be read anywhere inside.
            if self.scopes.iter().any(|sc| sc.fixed.iter().any(|n| &**n == name)) {
                return true;
            }
        }
        let info = self.root_info(self.scopes[at].root);
        if info.nonlocal.contains_key(name) {
            return true;
        }
        if let Some(named) = self.named_as.get(name) {
            for &t in named {
                if Some(t) == s || self.bound_name(t) != Some(name) {
                    continue;
                }
                // A local of an enclosing scope the root reads.
                if info.reads.contains_key(&t) && !info.declared.contains_key(&t) {
                    return true;
                }
                // A binder of the root around a read of the local.
                if let (Some(s), Some(&node)) = (s, info.node_of.get(&t)) {
                    if info.around(node, s) {
                        return true;
                    }
                }
            }
        }
        if let (Some(s), Some(nodes)) = (s, info.fixed_nodes.get(name)) {
            if nodes.iter().any(|&node| info.around(node, s)) {
                return true;
            }
        }
        false
    }

    /// Whether a binding named `name` of the instance of the object `c`, which `close_frame`
    /// declares in the scope at `at` and reads where the scope's text reads the instance, would
    /// read or be read for something else: what `captured` says of a temporary of the scope, or a
    /// binder of its root around a read of the instance, which would shadow the binding there.
    pub(super) fn binding_captured(&mut self, name: &str, c: ClassId, at: usize) -> bool {
        if self.captured(name, None, at) {
            return true;
        }
        let info = self.root_info(self.scopes[at].root);
        debug_assert!(info.modules.contains_key(&c), "the walk of {:?} reads no instance of the object it binds", self.scopes[at].root);
        // Where the walk did not find the reads, any binder of the name may stand around one.
        let around = |node: u32| !info.modules.contains_key(&c) || info.around_module(node, c);
        if let Some(named) = self.named_as.get(name) {
            if named.iter().any(|&t| self.bound_name(t) == Some(name) && info.node_of.get(&t).is_some_and(|&node| around(node))) {
                return true;
            }
        }
        info.fixed_nodes.get(name).is_some_and(|nodes| nodes.iter().any(|&node| around(node)))
    }

    /// The walk of a root, made once per item.
    fn root_info(&mut self, root: Root) -> Rc<RootInfo> {
        if let Some(info) = self.roots.get(&root) {
            return info.clone();
        }
        let mut w = Walk { info: RootInfo::default(), top: NONE, closure: self.closure_captures.clone() };
        let outer = std::mem::replace(&mut self.walking, true);
        // A jump of the loop being written assigns its parameters, a left-out argument from
        // the parameter's default.
        if let Some(t) = &self.tail {
            for p in t.params.clone() {
                w.name(&p);
            }
            if let Some(r) = t.receiver.clone() {
                w.name(&r);
            }
            for d in self.prog.funs[t.fun.idx()].defaults.clone().into_iter().flatten() {
                self.walk(&mut w, d);
            }
        }
        self.walk_root(&mut w, root);
        self.walking = outer;
        let n = w.info.binders.len();
        let mut end: Vec<u32> = (1..=n as u32).collect();
        for i in (0..n).rev() {
            let parent = w.info.binders[i].1;
            if parent != NONE {
                end[parent as usize] = end[parent as usize].max(end[i]);
            }
        }
        w.info.end = end;
        if cfg!(debug_assertions) {
            for n in w.info.nonlocal.keys() {
                debug_assert!(self.renderable.may_meet(n) || self.open_names.contains_key(n), "the walk of {:?} finds `{}`, which no stem may meet", root, n);
            }
        }
        let info = Rc::new(w.info);
        self.roots.insert(root, info.clone());
        info
    }

    /// Checks, in a build with assertions, that a name the emitter writes as itself in the scope
    /// being written is one its walk found: the walk and the emission agree.
    pub(super) fn note_name(&mut self, name: &str) {
        if !cfg!(debug_assertions) || self.scopes.is_empty() {
            return;
        }
        let root = self.scopes.last().unwrap().root;
        let info = self.root_info(root);
        debug_assert!(info.nonlocal.contains_key(name), "`{}` is written in {:?} but its walk did not find it", name, root);
        debug_assert!(self.renderable.may_meet(name), "`{}` is written but its stem is not among the rendered ones", name);
    }

    /// `note_name` for every free identifier of a fixed text.
    pub(super) fn note_text(&mut self, text: &str) {
        if !cfg!(debug_assertions) || text.is_empty() {
            return;
        }
        for name in text_names(text).free {
            self.note_name(&name);
        }
    }

    fn walk_root(&mut self, w: &mut Walk, root: Root) {
        let prog = self.prog;
        match root {
            Root::Expr(e) => self.walk(w, e),
            Root::Getter(e) => {
                self.walk_text(w, super::LAZY_FIELD);
                self.walk(w, e);
            }
            Root::Fun(f) => {
                let fun = &prog.funs[f.idx()];
                if matches!(self.syms.sym(fun.sym).owner, Owner::Package(_)) && self.reach.forwarded.get(fun.sym.idx()).copied().unwrap_or(false) {
                    if let Some(init) = self.needs_init(fun.sym, None).and_then(|file| self.file_init_text(file)) {
                        w.name(&init);
                    }
                }
                self.walk_fun(w, f);
            }
            Root::Lambda(e) => {
                let TExpr::Lambda(params, body) = prog.expr(e) else { unreachable!("a lambda") };
                for &p in prog.sym_list(params) {
                    w.bind(p);
                }
                self.walk(w, body);
            }
            Root::Closure(e) => self.walk_closure(w, e, false),
            Root::Cases(start, len) => {
                for i in start..start + len {
                    self.walk_case(w, i);
                }
            }
            Root::Case(i) => self.walk_case(w, i),
            Root::Ctor(idx) => self.walk_ctor(w, idx as usize),
            Root::Vals => {
                for e in self.group_inits.clone() {
                    self.walk(w, e);
                }
            }
        }
    }

    fn walk_list(&mut self, w: &mut Walk, l: ListRef) {
        for &a in self.prog.expr_list(l) {
            self.walk(w, a);
        }
    }

    /// The arguments of a call, whose literal varargs are passed as plain arguments.
    fn walk_args(&mut self, w: &mut Walk, l: ListRef) {
        for &a in self.prog.expr_list(l) {
            let literal = match self.peek(a) {
                TExpr::Spread(inner) => self.seq_literal_items(inner),
                _ => None,
            };
            match literal {
                Some(items) => {
                    for &item in items {
                        self.walk(w, item);
                    }
                }
                None => self.walk(w, a),
            }
        }
    }

    fn walk_fun(&mut self, w: &mut Walk, f: FunId) {
        let fun = &self.prog.funs[f.idx()];
        let mark = w.top;
        for &p in &fun.params {
            w.bind(p);
        }
        for d in fun.defaults.iter().flatten() {
            self.walk(w, *d);
        }
        if let Some(body) = fun.body {
            self.walk(w, body);
        }
        w.top = mark;
    }

    /// The arrow function of a lowered anonymous class: what it reads of the locals it captures
    /// is what its creation passes, and `inner` says that the creation is walked from outside.
    fn walk_closure(&mut self, w: &mut Walk, e: TExprId, inner: bool) {
        let prog = self.prog;
        let TExpr::New(c, args) = prog.expr(e) else { unreachable!("a creation") };
        let tc = &prog.classes[self.closure_anons[&c]];
        let fun = &prog.funs[tc.methods[0].idx()];
        let Some(body) = fun.body else { return };
        if let (true, Some((n, HoleKind::Thunk))) = (fun.params.is_empty(), self.hole_of(body)) {
            w.name(&format!("$k{}", n));
            return;
        }
        let mark = w.closure.len();
        if inner {
            for (&s, &a) in tc.ctor_params.iter().zip(prog.expr_list(args)) {
                if !matches!(prog.expr(a), TExpr::Local(t) if t == s) {
                    w.closure.push((s, a));
                }
            }
        }
        let binders = w.top;
        for &p in &fun.params {
            w.bind(p);
        }
        for d in fun.defaults.iter().flatten() {
            self.walk(w, *d);
        }
        self.walk(w, body);
        w.top = binders;
        w.closure.truncate(mark);
    }

    fn walk_case(&mut self, w: &mut Walk, i: u32) {
        let case = self.prog.cases[i as usize];
        let mark = w.top;
        let mut binders = Vec::new();
        self.pattern_binders(case.pat, &mut binders);
        for b in binders {
            w.bind(b);
        }
        self.walk_pat(w, case.pat);
        if let Some(g) = case.guard {
            self.walk(w, g);
        }
        self.walk(w, case.body);
        w.top = mark;
    }

    fn walk_pat(&mut self, w: &mut Walk, pat: TPatId) {
        let prog = self.prog;
        match prog.pats[pat.idx()] {
            TPat::Wildcard => {}
            TPat::Bind(_, inner) => {
                if let Some(i) = inner {
                    self.walk_pat(w, i);
                }
            }
            TPat::Test(test, _, inner) => {
                match self.test_hole(HoleNode::TestPat(pat)) {
                    Some(n) => w.name(&format!("$k{}", n)),
                    None => self.walk_test(w, test),
                }
                self.walk_pat(w, inner);
            }
            TPat::Equals(e, strict) => {
                if !strict {
                    w.name("$eq");
                }
                self.walk(w, e);
            }
            TPat::Class(c, _, _, subs) => {
                self.walk_class(w, c);
                for &s in &prog.pat_lists[subs.range()] {
                    self.walk_pat(w, s);
                }
            }
            TPat::Alt(subs) => {
                for &s in &prog.pat_lists[subs.range()] {
                    self.walk_pat(w, s);
                }
            }
            TPat::Unapply(_, call, inner) => {
                self.walk(w, call);
                self.walk_pat(w, inner);
            }
            TPat::Seq(items, rest) => {
                w.name("$seqPat");
                if let (Some(_), Some(c)) = (rest, self.array_seq) {
                    self.walk_class(w, c);
                }
                for &s in prog.pat_lists[items.range()].iter().chain(rest.iter()) {
                    self.walk_pat(w, s);
                }
            }
        }
    }

    fn walk_test(&mut self, w: &mut Walk, test: TestId) {
        let t = self.prog.tests[test.idx()];
        if let Some((text, _)) = test_text(t) {
            for name in text_names(text).free {
                w.name(&name);
            }
            return;
        }
        match t {
            TypeTest::Class(c) => {
                self.walk_class(w, c);
            }
            TypeTest::Value(v) => self.walk(w, v),
            TypeTest::Or(a, b) | TypeTest::And(a, b) => {
                self.walk_test(w, a);
                self.walk_test(w, b);
            }
            _ => {}
        }
    }

    fn walk_text(&mut self, w: &mut Walk, text: &str) {
        if !text.is_empty() {
            for name in text_names(text).free {
                w.name(&name);
            }
        }
    }

    /// A read of the local or the definition `s` where `local_ref` writes it.
    fn walk_ref(&mut self, w: &mut Walk, s: SymId) {
        if let Some(&(_, passed)) = w.closure.iter().rev().find(|&&(c, _)| c == s) {
            match self.prog.expr(passed) {
                TExpr::Local(t) => self.walk_ref(w, t),
                _ => w.this(self.this_name),
            }
            return;
        }
        if !self.body_params.is_empty() && self.body_params.contains(&s) && !self.captures.contains(&s) {
            w.this(self.this_name);
            return;
        }
        if let Some(i) = self.captures.iter().position(|&c| c == s) {
            if self.before_super {
                w.name(&format!("$c{}", i));
            } else {
                w.this(self.this_name);
            }
            return;
        }
        match self.syms.sym(s).owner {
            Owner::Package(_) => {
                let n = self.sym_text(s);
                w.name(&n);
            }
            // A constructor's parameter, which the constructor reads as its JS parameter.
            _ => w.read(s),
        }
    }

    fn walk_class(&mut self, w: &mut Walk, c: ClassId) {
        let n = self.class_text(c);
        w.name(identifier_of(&n));
    }

    fn walk_static(&mut self, w: &mut Walk, s: SymId) {
        let info = self.syms.sym(s);
        // A local object's value is a local of the output.
        if info.owner == Owner::Local {
            w.read(s);
            return;
        }
        if let SymKind::EnumValue(case) = info.kind {
            match info.owner {
                Owner::Class(companion) if self.enum_of_case(case).map_or(false, |e| e.stateful) || self.has_body[companion.idx()] => {
                    self.walk_accessor(w, companion);
                }
                _ => {}
            }
            if !matches!(info.owner, Owner::Class(_)) || !self.enum_of_case(case).map_or(false, |e| e.stateful) {
                let n = self.enum_value_text(s);
                w.name(&n);
            }
            return;
        }
        let n = self.sym_text(s);
        w.name(&n);
    }

    fn walk_accessor(&mut self, w: &mut Walk, c: ClassId) {
        let accessor = self.accessor_text(c);
        w.module(c, &accessor);
    }

    fn walk_module(&mut self, w: &mut Walk, c: ClassId) {
        match self.syms.class(c).js_binding {
            Some(JsBinding::Import(i)) => w.name(&format!("$imp{}", i)),
            Some(JsBinding::Global(n)) => self.walk_global(w, n),
            Some(JsBinding::GlobalScope) => w.name("globalThis"),
            None => self.walk_accessor(w, c),
        }
    }

    fn walk_global(&mut self, w: &mut Walk, name: crate::intern::Name) {
        let text = self.interner.get(name);
        w.name(if is_js_identifier(text) { text } else { "globalThis" });
    }

    fn walk_block(&mut self, w: &mut Walk, stmts: ListRef, res: Option<TExprId>) {
        let prog = self.prog;
        let items = &prog.stmts[stmts.range()];
        let mark = w.top;
        let mut binders = Vec::new();
        for s in items {
            match *s {
                TStmt::Val(sym, _) => binders.push(sym),
                TStmt::Fun(f) => binders.push(prog.funs[f.idx()].sym),
                TStmt::Pat(p, _) => self.pattern_binders(p, &mut binders),
                TStmt::Expr(_) => {}
            }
        }
        for b in binders {
            w.bind(b);
        }
        for s in items {
            match *s {
                TStmt::Fun(f) => self.walk_fun(w, f),
                TStmt::Expr(x) if self.is_idle(x) => {}
                TStmt::Expr(x) => self.walk(w, x),
                TStmt::Val(_, init) => self.walk(w, init),
                TStmt::Pat(p, init) => {
                    w.name("$matchError");
                    self.walk(w, init);
                    self.walk_pat(w, p);
                }
            }
        }
        if let Some(r) = res {
            self.walk(w, r);
        }
        w.top = mark;
    }

    fn walk_ctor(&mut self, w: &mut Walk, idx: usize) {
        let prog = self.prog;
        let tc = &prog.classes[idx];
        let info = self.syms.class(tc.id);
        for text in self.ctor_texts.clone() {
            self.walk_text(w, text);
        }
        self.walk_class(w, tc.id);
        if info.kind == ClassKind::Object {
            let accessor = self.accessor_text(tc.id);
            w.name(&super::instance_of(&accessor));
        }
        if let Some(s) = info.superclass {
            self.walk_class(w, s);
        }
        for &p in &tc.ctor_params {
            w.bind(p);
        }
        for d in tc.ctor_defaults.iter().flatten() {
            self.walk(w, *d);
        }
        self.walk_block(w, tc.parent_prelude, None);
        if let Some(args) = tc.parent_args {
            self.walk_list(w, args);
        }
        if let Some(via) = tc.parent_via {
            for f in super::layout::ctor_chain(prog, self.syms, via) {
                let fun = &prog.funs[f.idx()];
                for d in fun.defaults.iter().flatten() {
                    self.walk(w, *d);
                }
                if let Some(TExpr::Block(stmts, _)) = fun.body.map(|b| prog.expr(b)) {
                    self.walk_block(w, stmts, None);
                }
            }
        }
        if let Some(e) = info.companion.map(|e| self.syms.class(e)).filter(|e| e.stateful) {
            for &case in &e.children.clone() {
                self.walk_class(w, case);
            }
        }
        for init in &tc.init {
            match init {
                TInit::Field(s, e) => {
                    if self.syms.sym(*s).overridden_by_param {
                        self.walk_text(w, super::OWN_FIELD);
                    }
                    self.walk(w, *e);
                }
                TInit::Stmt(e) => self.walk(w, *e),
                TInit::Parent(b, call) => {
                    self.walk_class(w, *b);
                    self.walk_block(w, call.prelude, None);
                    self.walk_list(w, call.args);
                }
            }
        }
    }

    fn walk_site(&mut self, w: &mut Walk, site: u32) {
        let outline = self.outline;
        let f = outline.site_fun[site as usize];
        w.name(&outline.funs[f as usize].name);
        for hole in outline.site_holes[site as usize].iter() {
            match (hole.kind, hole.node) {
                (_, HoleNode::Expr(x)) => self.walk(w, x),
                (_, HoleNode::TestExpr(e)) => {
                    if let TExpr::TypeTest(_, t) = self.prog.expr(e) {
                        w.bind_fixed("$v");
                        self.walk_test(w, t);
                        w.pop();
                    }
                }
                (_, HoleNode::TestPat(p)) => {
                    if let TPat::Test(t, _, _) = self.prog.pats[p.idx()] {
                        w.bind_fixed("$v");
                        self.walk_test(w, t);
                        w.pop();
                    }
                }
            }
        }
    }

    /// The names `emit_expr_as` and the statements write for `e`, with the binders around the
    /// reads of locals.
    fn walk(&mut self, w: &mut Walk, e: TExprId) {
        if let Some((n, _)) = self.hole_of(e) {
            w.name(&format!("$k{}", n));
            return;
        }
        if let Some(site) = self.site_of(e) {
            self.walk_site(w, site);
            return;
        }
        if let Some(init) = self.init_before(e).and_then(|file| self.file_init_text(file)) {
            w.name(&init);
        }
        let prog = self.prog;
        match prog.expr(e) {
            TExpr::Int(_) | TExpr::Long(_) | TExpr::Bool(_) | TExpr::Char(_) | TExpr::Str(_) | TExpr::Unit | TExpr::Null | TExpr::Splice(_) => {}
            TExpr::Double(v) => {
                if let Some(g) = double_global(v) {
                    w.name(g);
                }
            }
            TExpr::Local(s) => self.walk_ref(w, s),
            TExpr::This | TExpr::Super(_) => w.this(self.this_name),
            TExpr::Static(s) => self.walk_static(w, s),
            TExpr::Module(c) => self.walk_module(w, c),
            TExpr::ClassOf(c) => {
                for helper in ["$traitClass", "$boxedClass", "$classValue", "$classNamed", "$classData"] {
                    w.name(helper);
                }
                if self.syms.class(c).kind != ClassKind::Builtin && self.syms.class(c).js == JsKind::Scala && self.reach.classes.get(c.idx()).copied().unwrap_or(false) {
                    self.walk_class(w, c);
                }
            }
            TExpr::Field(r, s) if self.syms.js_member(s) => match self.global_scope_member(r, s) {
                Some(_) => self.walk_global(w, self.syms.js_member_name(s)),
                None => {
                    self.walk(w, r);
                    if self.syms.sym(s).js_symbol {
                        w.name("Symbol");
                    }
                }
            },
            TExpr::Field(r, s) if matches!(self.peek(r), TExpr::This) && self.ctor_locals.contains(&s) => w.read(s),
            TExpr::Field(r, _) | TExpr::Index(r, _) | TExpr::Spread(r) | TExpr::Return(r) => self.walk(w, r),
            TExpr::CallStatic(s, args) => {
                self.walk_ref(w, s);
                self.walk_args(w, args);
            }
            TExpr::CallMethod(r, s, args) => {
                match prog.expr(r) {
                    TExpr::Super(target) => {
                        w.this(self.this_name);
                        if let SuperTarget::Class(c) = target {
                            self.walk_class(w, c);
                        }
                    }
                    _ if self.syms.js_member(s) => match self.global_scope_member(r, s) {
                        Some(_) => self.walk_global(w, self.syms.js_member_name(s)),
                        None => {
                            self.walk(w, r);
                            if self.syms.sym(s).js_symbol {
                                w.name("Symbol");
                            }
                        }
                    },
                    _ => self.walk(w, r),
                }
                self.walk_args(w, args);
            }
            TExpr::CallClosure(f, args) => {
                self.walk(w, f);
                self.walk_args(w, args);
            }
            TExpr::New(c, _) if self.closure_anons.contains_key(&c) => self.walk_closure(w, e, true),
            TExpr::New(c, _) if self.syms.class(c).kind == ClassKind::Builtin => {}
            TExpr::NewVia(s, args) => {
                if let Owner::Class(c) = self.syms.sym(s).owner {
                    self.walk_class(w, c);
                }
                self.walk_args(w, args);
            }
            TExpr::New(c, args) => {
                if let Some(companion) = self.companion_touch(c) {
                    self.walk_accessor(w, companion);
                }
                self.walk_class(w, c);
                if self.syms.class(c).kind == ClassKind::Anon || self.local_captures.contains_key(&c) {
                    self.walk_capture_args(w, c, args);
                } else {
                    self.walk_args(w, args);
                }
            }
            TExpr::Lambda(params, body) => {
                if let Some(target) = self.forwarded_function(params, body) {
                    self.walk_ref(w, target);
                    return;
                }
                if let (true, Some((n, HoleKind::Thunk))) = (params.is_empty(), self.hole_of(body)) {
                    w.name(&format!("$k{}", n));
                    return;
                }
                let mark = w.top;
                for &p in prog.sym_list(params) {
                    w.bind(p);
                }
                self.walk(w, body);
                w.top = mark;
            }
            TExpr::If(c, t, els) => {
                self.walk(w, c);
                self.walk(w, t);
                if let Some(x) = els {
                    self.walk(w, x);
                }
            }
            TExpr::While(c, body) => {
                self.walk(w, c);
                self.walk(w, body);
            }
            TExpr::Block(stmts, res) => self.walk_block(w, stmts, Some(res)),
            TExpr::Assign(target, value) => {
                match prog.expr(target) {
                    TExpr::Static(s) if self.syms.sym(s).owner == Owner::Local => w.read(s),
                    TExpr::Static(s) => {
                        let n = self.sym_text(s);
                        w.name(&format!("{}set", n));
                        if let Some(init) = self.needs_init(s, None).and_then(|file| self.file_init_text(file)) {
                            w.name(&init);
                        }
                    }
                    _ => self.walk(w, target),
                }
                self.walk(w, value);
            }
            TExpr::Match(scrut, cases) => {
                w.name("$matchError");
                self.walk(w, scrut);
                for i in cases.start..cases.start + cases.len {
                    self.walk_case(w, i);
                }
            }
            TExpr::Try(i) => {
                let t = &prog.tries[i as usize];
                self.walk(w, t.body);
                if t.wraps {
                    w.name("$wrapJs");
                }
                for c in t.cases.start..t.cases.start + t.cases.len {
                    self.walk_case(w, c);
                }
                if let Some(f) = t.finalizer {
                    self.walk(w, f);
                }
            }
            TExpr::Prim(op, a, b) => {
                let (pre, _, _) = prim_text(op, self.int_literal_nonzero(b));
                self.walk_text(w, pre);
                self.walk(w, a);
                self.walk(w, b);
            }
            TExpr::Unary(op, a) => {
                self.walk_text(w, unary_text(op).0);
                self.walk(w, a);
            }
            // An operand that JavaScript's `+` turns into a string as Scala does is written as is.
            TExpr::StrConcat(items) => {
                for &item in prog.expr_list(items) {
                    match self.peek(item) {
                        TExpr::ToStr(inner, conv) if conv.kind() == StrKind::Plain => self.walk(w, inner),
                        _ => self.walk(w, item),
                    }
                }
            }
            TExpr::ArrayLit(items) | TExpr::ObjLit(items) => self.walk_list(w, items),
            TExpr::SeqLit(items) => {
                if let Some(c) = self.array_seq {
                    self.walk_class(w, c);
                }
                self.walk_list(w, items);
            }
            TExpr::ToStr(inner, conv) => {
                self.walk_text(w, to_str_text(conv.kind()));
                self.walk(w, inner);
            }
            TExpr::Js(t, args) => {
                let computed;
                let names = match self.renderable.template(&prog.strings[t.idx()]) {
                    Some(n) => n,
                    None => {
                        computed = text_names(&prog.strings[t.idx()]);
                        &computed
                    }
                };
                let free: Vec<String> = names.free.clone();
                let around: Vec<(u8, Vec<String>)> = names.around.clone();
                let args = prog.expr_list(args);
                for name in &free {
                    w.name(name);
                }
                // An argument the template has no placeholder for is not written.
                for (k, names) in &around {
                    let Some(&a) = args.get(*k as usize) else { continue };
                    let mark = w.top;
                    for n in names {
                        w.bind_fixed(n);
                    }
                    // A spread of literal varargs is written as its elements.
                    let spread = prog.strings[t.idx()].contains(&format!("...${}", k));
                    match self.seq_literal_items(a).filter(|_| spread) {
                        Some(items) => {
                            for &item in items {
                                self.walk(w, item);
                            }
                        }
                        None => self.walk(w, a),
                    }
                    w.top = mark;
                }
            }
            TExpr::TypeTest(inner, test) => {
                match self.test_hole(HoleNode::TestExpr(e)) {
                    Some(n) => w.name(&format!("$k{}", n)),
                    // A tested value other than a local is passed to `($v) => test`.
                    None if !matches!(self.peek(inner), TExpr::Local(_)) => {
                        w.bind_fixed("$v");
                        self.walk_test(w, test);
                        w.pop();
                    }
                    None => self.walk_test(w, test),
                }
                self.walk(w, inner);
            }
            TExpr::Cast(inner, op, _) => {
                match op {
                    CastOp::Written => {}
                    CastOp::Nothing => w.name("$asNothing"),
                    CastOp::Unbox(test, _) => w.name(super::expr::unbox_helper(prog.tests[test.idx()])),
                    CastOp::Check(test, _) => match prog.tests[test.idx()] {
                        TypeTest::Class(c) => {
                            w.name("$as");
                            self.walk_class(w, c);
                        }
                        TypeTest::Trait(_) => w.name("$asA"),
                        TypeTest::Str => w.name("$asS"),
                        // The test is written on the local, or on `$v` of `($v) => $asT(..)`.
                        _ if matches!(self.peek(inner), TExpr::Local(_)) => {
                            w.name("$asT");
                            self.walk_test(w, test);
                        }
                        _ => {
                            w.name("$asT");
                            w.bind_fixed("$v");
                            self.walk_test(w, test);
                            w.pop();
                        }
                    },
                }
                self.walk(w, inner);
            }
            TExpr::JsImport(i) => w.name(&format!("$imp{}", i)),
            TExpr::JsGlobal(name, _) => self.walk_global(w, name),
            TExpr::JsSelect(r, name) if self.is_global_scope(r) => self.walk_global(w, name),
            TExpr::JsSelect(r, _) => self.walk(w, r),
            TExpr::Throw(inner, unwrap) => {
                w.name(THROW);
                if unwrap {
                    w.name(UNWRAP_JS);
                }
                self.walk(w, inner);
            }
        }
    }

    /// What `emit_capture_args` writes: a captured `var` the scope does not hold in a field is
    /// passed as a cell whose setter binds `$v` around a write of it.
    fn walk_capture_args(&mut self, w: &mut Walk, c: ClassId, args: ListRef) {
        let captures = match self.local_captures.get(&c) {
            Some(&n) => n as usize,
            None => (args.len - self.anon_parent_args.get(&c).copied().unwrap_or(0)) as usize,
        };
        for (i, &a) in self.prog.expr_list(args).iter().enumerate() {
            match self.peek(a) {
                TExpr::Local(s) if i < captures && self.syms.sym(s).kind == SymKind::Var && !self.captures.contains(&s) => {
                    w.name("$ref");
                    self.walk_ref(w, s);
                    w.bind_fixed("$v");
                    self.walk_ref(w, s);
                    w.pop();
                }
                _ => self.walk(w, a),
            }
        }
    }
}
