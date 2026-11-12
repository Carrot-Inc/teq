//! SemanticDB's symbols at the names a rule of `teq fix` asks about (docs/FIX.md, "The rule API"): the port of dotty
//! 3.8.4's `SemanticSymbolBuilder.symbolName` (SemanticSymbolBuilder.scala 204-286: `addName`, `addOwner`,
//! `addOverloadIdx`, `addDescriptor`) and of `Scala3.isGlobal` (Scala3.scala 258-268) over what the typer resolved
//! each name to, the navigation index's records with SemanticDB's own (`Worker::sem_records`) and the declarations of the
//! document.
//!
//! What is answered is what the three semantic rules teq ships ask: the symbols of the occurrences at a name's exact
//! range, compared normalized (the owner chain's names) or, for a method of an object, exactly. A local symbol is
//! `local<n>`, numbered in the order a document's questions meet it, not in the order of dotty's traversal: no rule
//! can tell (`SymbolOps.normalize` leaves a local as it is, and no entry names one); the parameters the document
//! declares at one place, which the methods of a local extension group each have, are one local, as dotty's `localIdx`
//! numbers them. A parameter and a type parameter
//! are their owner's (`m().(x)`, `C#[T]`) where the owner is a definition of the document or, for a parameter, a
//! method of the class that owns it; else they are answered as locals.

use super::super::index::{Kind, Target};
use super::super::Worker;
use crate::intern::FxMap;
use crate::source::{FileId, Span};
use crate::symbols::*;
use crate::types::*;

/// An occurrence at a range: its symbol, and whether it is the symbol's definition (`SymbolOccurrence.Role`).
pub struct Occurrence {
    pub symbol: String,
    pub definition: bool,
}

/// What a document's answers keep from one question to the next: its declarations by their names' spans, and the
/// numbers of its locals.
#[derive(Default)]
pub struct DocTable {
    entries: Vec<FileId>,
    decls: Vec<(Span, Target)>,
    /// The packages its paths name (`Index::packages`).
    packages: Vec<(Span, PkgId)>,
    locals: FxMap<Target, u32>,
    /// The number of the local parameter the document declares at a place (the file and the span of its name, as the
    /// index recorded the declaration): the parameters that the methods of a local extension group each have are one
    /// local, as dotty numbers a unit's local symbols of one name and one starting position
    /// (`SemanticSymbolBuilder.localIdx`). A parameter declared outside the document has no place here and keeps a
    /// number of its own.
    local_places: FxMap<(FileId, u32, u32), u32>,
    /// What each symbol answered stands for, which a later question about the symbol reads (`sem_overridden_of`).
    named: FxMap<String, Target>,
    /// The method each parameter of the document's methods belongs to, and the definition each of its type
    /// parameters does.
    param_owner: FxMap<SymId, SymId>,
    tparam_owner: FxMap<TParamId, Target>,
}

/// What the documents of one typing share: its files' declarations and the packages their paths name, found once.
pub struct Declared {
    decls: FxMap<FileId, Vec<(Span, Target)>>,
    packages: FxMap<FileId, Vec<(Span, PkgId)>>,
}

impl Declared {
    pub fn of(w: &Worker<'_>) -> Declared {
        Declared { decls: w.declarations_by_file(), packages: w.packages_by_file() }
    }
}

impl DocTable {
    /// The table of the document whose entries (the file's own and its package blocks') are `entries`.
    pub fn of(w: &mut Worker<'_>, declared: &Declared, entries: Vec<FileId>) -> DocTable {
        let mut decls: Vec<(Span, Target)> = entries.iter().filter_map(|f| declared.decls.get(f)).flatten().copied().collect();
        decls.sort_by_key(|(span, t)| (span.start, span.end, *t));
        decls.dedup();
        let (mut param_owner, mut tparam_owner) = (FxMap::default(), FxMap::default());
        for &(_, t) in &decls {
            match t {
                Target::Sym(s) if matches!(w.syms.sym(s).kind, SymKind::Def | SymKind::Given) => {
                    let sig = w.sig_arc(s);
                    for p in sig.clauses.iter().flat_map(|c| c.params.iter()) {
                        param_owner.entry(p.sym).or_insert(s);
                    }
                    for &tp in &sig.tparams {
                        tparam_owner.entry(tp).or_insert(t);
                    }
                }
                Target::Class(c) => {
                    for &tp in &w.syms.class(c).tparams {
                        tparam_owner.entry(tp).or_insert(t);
                    }
                }
                Target::Alias(a) => {
                    for &tp in &w.syms.aliases[a.idx()].tparams {
                        tparam_owner.entry(tp).or_insert(t);
                    }
                }
                _ => {}
            }
        }
        // An extension group's parameter: every method of the group has its own, declared at the group's one place
        // (dotty gives each method the group's clauses), so the place holds a definition a method, in the methods'
        // order; a use names its own method's (`Worker::normal_target`). Read off the methods' signatures, not off
        // the index, whose one parameter a place is navigation's.
        let methods: Vec<SymId> = decls.iter().filter_map(|&(_, t)| match t {
            Target::Sym(s) if w.syms.sym(s).kind == SymKind::Def && w.syms.sym(s).ext_clauses > 0 => Some(s),
            _ => None,
        }).collect();
        let mut group: Vec<(Span, Target)> = Vec::new();
        for m in methods {
            let leading = w.syms.sym(m).ext_clauses as usize;
            let sig = w.sig_arc(m);
            for p in sig.clauses.iter().take(leading).flat_map(|c| c.params.iter()) {
                // The parameter's own declaration in the document, as the index recorded it: an export's forwarder of
                // an extension method has the clauses too, and its parameters are declared nowhere.
                if let Some((_, span)) = w.param_declared(p.sym).filter(|(file, _)| entries.contains(file)) {
                    group.push((span, Target::Sym(p.sym)));
                }
            }
        }
        decls.extend(group);
        // Stable: at one place the declarations stay in their order, the group's parameters in their methods'.
        decls.sort_by_key(|(span, _)| (span.start, span.end));
        decls.dedup();
        let mut packages: Vec<(Span, PkgId)> = entries.iter().filter_map(|f| declared.packages.get(f)).flatten().copied().collect();
        packages.sort_by_key(|(span, p)| (span.start, span.end, p.0));
        packages.dedup();
        DocTable { entries, decls, packages, locals: FxMap::default(), local_places: FxMap::default(), named: FxMap::default(), param_owner, tparam_owner }
    }
}

impl<'a> Worker<'a> {
    /// The occurrences at exactly `span` of the document: the declarations whose name stands there, then what the
    /// name there was resolved to (`ExtractSemanticDB`'s `registerDefinition` and `registerUse`). None where no name
    /// stands; an error where the typing kept no index to answer from, or a symbol cannot be named.
    pub fn sem_occurrences(&mut self, table: &mut DocTable, span: Span) -> Result<Vec<Occurrence>, String> {
        if self.index.is_none() {
            return Err("the typing kept no index of the names it resolved (TEQ_FIX_INDEX=off), so no symbol is answered".to_string());
        }
        let mut out: Vec<Occurrence> = Vec::new();
        if span.end <= span.start {
            return Ok(out);
        }
        // The records by span, as a query of the index reads them.
        self.index_order();
        let from = table.decls.partition_point(|(s, _)| (s.start, s.end) < (span.start, span.end));
        let declared: Vec<Target> = table.decls[from..].iter().take_while(|(s, _)| *s == span).map(|&(_, t)| t).collect();
        for t in declared {
            let symbol = self.sem_symbol(table, t)?;
            if !out.iter().any(|o| o.symbol == symbol) {
                out.push(Occurrence { symbol, definition: true });
            }
        }
        let from = table.packages.partition_point(|(s, _)| (s.start, s.end) < (span.start, span.end));
        // A package a path or a selector names; at a selector's renamed name it is no occurrence, as a member's is none.
        if let Some(&(_, p)) = table.packages.get(from).filter(|(s, _)| *s == span) {
            let spelled = p == ROOT_PKG || table.entries.first().is_some_and(|&file| {
                self.source(file).text.get(span.start as usize..span.end as usize).is_some_and(|written| written.trim_matches('`') == self.name_ref(self.syms.pkg(p).name))
            });
            if spelled {
                let mut symbol = String::new();
                self.sem_add_pkg(&mut symbol, p);
                out.push(Occurrence { symbol, definition: false });
            }
        }
        if let Some((t, kind)) = self.index_record_at(&table.entries, span) {
            // An import's renamed name is no occurrence: dotty records the selector's own name alone
            // (`ExtractSemanticDB.traverse`, 423-437), and the index both for a rename.
            let renamed = kind == Kind::Import && !self.sem_spelled(&table.entries, span, t);
            if !renamed && matches!(kind, Kind::Ref | Kind::Call | Kind::Type | Kind::Import | Kind::Pattern | Kind::NamedArg | Kind::Def) && !matches!(t, Target::Node(_)) {
                let symbol = self.sem_symbol(table, t)?;
                if !out.iter().any(|o| o.symbol == symbol) {
                    out.push(Occurrence { symbol, definition: kind == Kind::Def });
                }
            }
        }
        Ok(out)
    }

    /// Every range of the document an occurrence stands at, with its occurrences, in the order of the ranges: what
    /// a document's rules ask name by name, made once (a rule asks at every name).
    pub fn sem_table(&mut self, table: &mut DocTable) -> Result<Vec<(Span, Vec<Occurrence>)>, String> {
        if self.index.is_none() {
            return Err("the typing kept no index of the names it resolved (TEQ_FIX_INDEX=off), so no symbol is answered".to_string());
        }
        self.index_order();
        let mut spans: Vec<Span> = table.decls.iter().map(|&(s, _)| s).chain(table.packages.iter().map(|&(s, _)| s)).collect();
        for &f in &table.entries.clone() {
            spans.extend(self.record_spans_of(f));
        }
        spans.sort_by_key(|s| (s.start, s.end));
        spans.dedup();
        let mut out = Vec::with_capacity(spans.len());
        for span in spans {
            let found = self.sem_occurrences(table, span)?;
            if !found.is_empty() {
                out.push((span, found));
            }
        }
        Ok(out)
    }

    /// Whether the text at `span` of the document spells the name of `t`, its backquotes aside.
    fn sem_spelled(&self, entries: &[FileId], span: Span, t: Target) -> bool {
        let name = match t {
            Target::Sym(s) => self.syms.sym(s).name,
            Target::Class(c) => self.syms.class(c).name,
            Target::Alias(a) => self.syms.aliases[a.idx()].name,
            Target::TParam(p) => self.syms.tparam(p).name,
            Target::Node(_) => return false,
        };
        let Some(&file) = entries.first() else { return false };
        let text = &self.source(file).text;
        text.get(span.start as usize..span.end as usize).is_some_and(|written| written.trim_matches('`') == self.name_ref(name))
    }

    /// `symbolName` of what `t` stands for.
    pub(super) fn sem_symbol(&mut self, table: &mut DocTable, t: Target) -> Result<String, String> {
        // A parameter of a global method and a type parameter of a global definition are global, named after their
        // owner (`addDescriptor`: `(x)`, `[T]`).
        let owned = match t {
            Target::Sym(s) if self.syms.sym(s).kind == SymKind::Param => self.sem_param_method(table, s).map(|m| (Target::Sym(m), '(', ')', self.syms.sym(s).name)),
            Target::TParam(p) => table.tparam_owner.get(&p).map(|&o| (o, '[', ']', self.syms.tparam(p).name)),
            _ => None,
        };
        if let Some((owner, open, close, name)) = owned {
            if self.sem_global(owner) {
                let mut b = String::with_capacity(40);
                self.sem_add(&mut b, owner)?;
                b.push(open);
                add_name(&mut b, self.name_ref(name));
                b.push(close);
                table.named.entry(b.clone()).or_insert(t);
                return Ok(b);
            }
        }
        if !self.sem_global(t) {
            let place = match t {
                Target::Sym(s) if self.syms.sym(s).kind == SymKind::Param => {
                    self.param_declared(s).filter(|(file, _)| table.entries.contains(file)).map(|(file, span)| (file, span.start, span.end))
                }
                _ => None,
            };
            let known = table.locals.get(&t).copied().or_else(|| place.and_then(|p| table.local_places.get(&p).copied()));
            let next = table.locals.len() as u32;
            let number = known.unwrap_or(next);
            let b = format!("local{}", number);
            // What the local stands for, where it is first numbered: a local class's member is asked what it
            // overrides as a global one is.
            if known.is_none() {
                table.locals.insert(t, number);
                if let Some(p) = place {
                    table.local_places.insert(p, number);
                }
                table.named.insert(b.clone(), t);
            }
            return Ok(b);
        }
        let mut b = String::with_capacity(32);
        self.sem_add(&mut b, t)?;
        table.named.entry(b.clone()).or_insert(t);
        Ok(b)
    }

    /// The method the parameter `p` belongs to: one of the document's, else a method of the class that owns the
    /// parameter (a named argument's, of another file's method).
    fn sem_param_method(&mut self, table: &mut DocTable, p: SymId) -> Option<SymId> {
        if let Some(&m) = table.param_owner.get(&p) {
            return Some(m);
        }
        let Owner::Class(c) = self.syms.sym(p).owner else { return None };
        let members: Vec<SymId> = self.syms.class(c).member_order.clone();
        for m in members {
            if !matches!(self.syms.sym(m).kind, SymKind::Def | SymKind::Given) {
                continue;
            }
            let sig = self.sig_arc(m);
            if sig.clauses.iter().flat_map(|cl| cl.params.iter()).any(|q| q.sym == p) {
                table.param_owner.insert(p, m);
                return Some(m);
            }
        }
        None
    }

    /// The symbols of the members the member `symbol` overrides, `allOverriddenSymbols`' in their order: none for
    /// what is no member of a class. `symbol` is one the document's questions were answered with.
    pub fn sem_overridden_of(&mut self, table: &mut DocTable, symbol: &str) -> Result<Vec<String>, String> {
        // A package overrides nothing.
        if symbol.ends_with('/') {
            return Ok(Vec::new());
        }
        let Some(&t) = table.named.get(symbol) else { return Err(format!("{} is no symbol this document's names were answered with", symbol)) };
        let Target::Sym(s) = t else { return Ok(Vec::new()) };
        let overridden = self.sem_overridden(s);
        overridden.into_iter().map(|p| self.sem_symbol(table, Target::Sym(p))).collect()
    }

    /// `Scala3.isGlobal`: a package; else a symbol whose owner is a class, itself global.
    fn sem_global(&self, t: Target) -> bool {
        match t {
            Target::Class(c) => self.sem_global_owner(self.syms.class(c).owner),
            Target::Sym(s) => {
                let info = self.syms.sym(s);
                info.kind != SymKind::Param && self.sem_global_owner(info.owner)
            }
            Target::Alias(a) => self.sem_global_owner(self.syms.aliases[a.idx()].owner),
            Target::TParam(_) | Target::Node(_) => false,
        }
    }

    fn sem_global_owner(&self, owner: Owner) -> bool {
        match owner {
            Owner::Package(_) => true,
            Owner::Class(c) => self.sem_global_owner(self.syms.class(c).owner),
            Owner::Local => false,
        }
    }

    /// `addSymName` of a global symbol: its owner's name, then its descriptor.
    fn sem_add(&mut self, b: &mut String, t: Target) -> Result<(), String> {
        match t {
            Target::Class(c) => {
                let (owner, name, kind) = {
                    let info = self.syms.class(c);
                    (info.owner, info.name, info.kind)
                };
                // A class of the lean std that scala-library has in another package is named as the library names it.
                let library = match owner {
                    Owner::Package(p) if self.sem_lean_std(self.syms.class(c).file) => crate::tasty::write::std_library_name(&self.pkg_description(p), self.name_ref(name)),
                    _ => None,
                };
                match library {
                    Some(full) => add_path(b, &full, if kind == ClassKind::Object { '.' } else { '#' }),
                    None => {
                        self.sem_add_owner(b, owner)?;
                        // A class of a `package object`, which teq reads as a block of its package, is the object's.
                        if matches!(owner, Owner::Package(_)) && self.ast(self.syms.class(c).file).package_object && self.name_ref(name) != "package" {
                            b.push_str("package.");
                        }
                        add_name(b, self.name_ref(name));
                        // `addDescriptor`: a module class is its module, a term; every other class a type.
                        b.push(if kind == ClassKind::Object { '.' } else { '#' });
                    }
                }
                Ok(())
            }
            Target::Sym(s) => {
                let (owner, name, kind, file, java) = {
                    let info = self.syms.sym(s);
                    (info.owner, info.name, info.kind, info.file, info.java_defined)
                };
                if let SymKind::Object(c) = kind {
                    return self.sem_add(b, Target::Class(c));
                }
                // A top-level definition or an extension of the lean std is the member of scala-library it stands for,
                // where the std's naming has one (`tasty::write::std_target_by`, the table the TASTy writer selects by).
                if let Some(owner) = self.sem_std_owner(s) {
                    b.push_str(&owner);
                    add_name(b, self.name_ref(name));
                    b.push_str(if matches!(kind, SymKind::Def | SymKind::Given) { "()." } else { "." });
                    return Ok(());
                }
                match owner {
                    Owner::Package(p) => self.sem_add_file_module(b, p, file),
                    owner => self.sem_add_owner(b, owner)?,
                }
                add_name(b, self.name_ref(name));
                // `addDescriptor`: a method, or a variable that is not Java's, has its overload index; a value none.
                let method = match kind {
                    SymKind::Def => true,
                    SymKind::Var => !java,
                    SymKind::Given => {
                        let sig = self.sig_arc(s);
                        !sig.clauses.is_empty() || !sig.tparams.is_empty()
                    }
                    _ => false,
                };
                if method {
                    b.push('(');
                    self.sem_add_overload(b, s);
                    b.push_str(").");
                } else {
                    b.push('.');
                }
                Ok(())
            }
            Target::Alias(a) => {
                let (owner, name, file) = {
                    let info = &self.syms.aliases[a.idx()];
                    (info.owner, info.name, info.file)
                };
                match owner {
                    Owner::Package(p) => self.sem_add_file_module(b, p, file),
                    owner => self.sem_add_owner(b, owner)?,
                }
                add_name(b, self.name_ref(name));
                b.push('#');
                Ok(())
            }
            Target::TParam(_) | Target::Node(_) => Err("a symbol that is not global has no global name".to_string()),
        }
    }

    /// `addOwner`: the owner's own symbol name, the root's none.
    fn sem_add_owner(&mut self, b: &mut String, owner: Owner) -> Result<(), String> {
        match owner {
            Owner::Package(p) => {
                self.sem_add_pkg(b, p);
                Ok(())
            }
            Owner::Class(c) => self.sem_add(b, Target::Class(c)),
            Owner::Local => Err("a symbol that is not global has no global name".to_string()),
        }
    }

    /// Whether `file` is one of teq's own std, the lean std's or the builtin layer's.
    fn sem_lean_std(&self, file: FileId) -> bool {
        self.files.as_slice().get(file.0 as usize).is_some_and(|f| f.is_std) && !self.in_jar(file)
    }

    /// The owner's symbol of the member of scala-library a top-level definition or an extension of teq's std stands
    /// for: `scala/collection/StringOps#` for a string's `init`, `scala/Predef.` for `println`; none for a
    /// definition scala-library has no member for, or one of a class.
    fn sem_std_owner(&mut self, s: SymId) -> Option<String> {
        use crate::tasty::write::StdTarget;
        let (file, owner, name, is_extension) = {
            let info = self.syms.sym(s);
            (info.file, info.owner, info.name, info.is_extension)
        };
        if !self.sem_lean_std(file) {
            return None;
        }
        let top = match owner {
            Owner::Package(_) => true,
            Owner::Class(c) => self.name_ref(self.syms.class(c).name).ends_with("$package"),
            Owner::Local => false,
        };
        if !top {
            return None;
        }
        let stem = crate::tasty::write::file_stem(&self.source(file).path);
        let name = self.name_str(name);
        let receiver = if is_extension {
            let recv = self.sig_of(s).clauses.first().and_then(|c| c.params.first()).map(|p| p.ty)?;
            let recv = self.widen_lit(recv);
            Some(match self.types.get(recv) {
                Type::Class(c, _) => self.name_str(self.syms.class(c).name),
                Type::Param(_) => "*".to_string(),
                _ => return None,
            })
        } else {
            None
        };
        let mut b = String::new();
        match crate::tasty::write::std_target_by(&stem, &name, receiver.as_deref())? {
            StdTarget::Object { pkg, object } => add_path(&mut b, &format!("{}.{}", pkg, object), '.'),
            StdTarget::Summon | StdTarget::Conforms => add_path(&mut b, "scala.Predef", '.'),
            StdTarget::Receiver { pkg, class, .. } => add_path(&mut b, &format!("{}.{}", pkg, class), '#'),
            StdTarget::Wrapped { class: (pkg, class), .. } => add_path(&mut b, &format!("{}.{}", pkg, class), '#'),
            // The array's own class, `ArrayOps` or an `ArraySeq`, whichever scala-library has the member in first.
            StdTarget::Array => {
                let owner = ["scala.Array", "scala.collection.ArrayOps"].into_iter().find(|o| crate::tasty::write::std_shape_named(o, &name)).unwrap_or("scala.collection.mutable.ArraySeq");
                add_path(&mut b, owner, '#');
            }
        }
        Some(b)
    }

    /// The owner of a top-level definition of `file` in the package `p`: its file's module, `p/<File>$package.`; of
    /// a member of a `package object p`, which teq reads as a block of the package, the object, `p/package.`.
    fn sem_add_file_module(&self, b: &mut String, p: PkgId, file: FileId) {
        self.sem_add_pkg(b, p);
        if self.ast(file).package_object {
            b.push_str("package.");
            return;
        }
        let stem = std::path::Path::new(&self.source(file).path).file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
        add_name(b, &format!("{}$package", stem));
        b.push('.');
    }

    /// A package's name: `a/b/`, the empty package `_empty_/` (`Symbols.EmptyPackage`).
    fn sem_add_pkg(&self, b: &mut String, p: PkgId) {
        if p == ROOT_PKG {
            b.push_str("_empty_/");
            return;
        }
        let info = self.syms.pkg(p);
        if let Some(parent) = info.parent.filter(|&parent| parent != ROOT_PKG) {
            self.sem_add_pkg(b, parent);
        }
        add_name(b, self.name_ref(info.name));
        b.push('/');
    }

    /// `addOverloadIdx`: where the owner declares several methods of the name, the place of this one among them
    /// after the first, `+1`, `+2`, in the order of their declaration.
    fn sem_add_overload(&self, b: &mut String, s: SymId) {
        let info = self.syms.sym(s);
        let entry = match info.owner {
            Owner::Class(c) => self.syms.class(c).members.get(&info.name).copied(),
            Owner::Package(p) => self.syms.entry_named(self.interner, p, info.name).and_then(|e| e.term),
            Owner::Local => None,
        };
        let Some(alts) = entry.and_then(|e| self.syms.alternatives(e)) else { return };
        let methods: Vec<SymId> = alts.iter().copied().filter(|&a| matches!(self.syms.sym(a).kind, SymKind::Def | SymKind::Var | SymKind::Given)).collect();
        if let Some(i) = methods.iter().position(|&a| a == s) {
            if i > 0 {
                b.push('+');
                b.push_str(&i.to_string());
            }
        }
    }
}

/// A qualified name of scala-library as a symbol: its packages, an object among them where a segment is capitalized
/// (`scala.Predef.ArrowAssoc`), and its last name with `last`, `#` a class's and `.` an object's.
fn add_path(b: &mut String, full: &str, last: char) {
    let segments: Vec<&str> = full.split('.').collect();
    for (i, segment) in segments.iter().enumerate() {
        add_name(b, segment);
        if i + 1 == segments.len() {
            b.push(last);
        } else {
            b.push(if segment.chars().next().is_some_and(char::is_uppercase) { '.' } else { '/' });
        }
    }
}

/// `addName`: the name where it is a Java identifier, else between backquotes.
fn add_name(b: &mut String, name: &str) {
    let mut chars = name.chars();
    let java = match chars.next() {
        Some(c) => java_identifier_start(c) && chars.all(java_identifier_part),
        None => false,
    };
    if java {
        b.push_str(name);
    } else {
        b.push('`');
        b.push_str(name);
        b.push('`');
    }
}

/// `Character.isJavaIdentifierStart`: a letter, a letter number, a currency sign or a connecting punctuation, as the
/// interpreter's builtin of that name has it (`interp::builtins`).
fn java_identifier_start(c: char) -> bool {
    // A supplementary character is two surrogate units to dotty's and scalameta's tests, neither a letter.
    (c as u32) <= 0xFFFF && (c.is_alphabetic() || matches!(c, '_' | '$' | '\u{203F}' | '\u{2040}' | '\u{2054}' | '\u{FE33}' | '\u{FE34}' | '\u{FE4D}'..='\u{FE4F}' | '\u{FF3F}') || matches!(c as u32, 0xA2..=0xA5 | 0x20A0..=0x20CF))
}

/// `Character.isJavaIdentifierPart`: a UTF-16 unit by the JDK's table (`jvm::names`); a supplementary character is
/// two surrogate units, which are no part of one, as dotty's `isJavaIdent` and scalameta's `encode` test them.
fn java_identifier_part(c: char) -> bool {
    let n = c as u32;
    n <= 0xFFFF && crate::jvm::names::is_java_identifier_part(n as u16)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_as_scalameta_encodes_them() {
        let named = |n: &str| {
            let mut b = String::new();
            add_name(&mut b, n);
            b
        };
        assert_eq!(named("get"), "get");
        assert_eq!(named("$conforms"), "$conforms");
        assert_eq!(named("+"), "`+`");
        assert_eq!(named("<init>"), "`<init>`");
        assert_eq!(named("html_<^"), "`html_<^`");
        assert_eq!(named("1st"), "`1st`");
        assert_eq!(named(""), "``");
        assert_eq!(named("\u{1D518}get"), "`\u{1D518}get`");
    }

    #[test]
    fn library_paths_as_symbols() {
        let path = |full: &str, last: char| {
            let mut b = String::new();
            add_path(&mut b, full, last);
            b
        };
        assert_eq!(path("scala.collection.immutable.List", '#'), "scala/collection/immutable/List#");
        assert_eq!(path("scala.Predef", '.'), "scala/Predef.");
        assert_eq!(path("scala.Predef.ArrowAssoc", '#'), "scala/Predef.ArrowAssoc#");
        assert_eq!(path("scala.math.package", '.'), "scala/math/package.");
    }
}
