//! The imports that bring what a report says is missing (`Diagnostic::missing`), which a language
//! server offers as quick fixes: the answer to a session's `import-actions`.
//!
//! A name not found is completion's own question asked for the whole name: the names out of the
//! scope of that spelling (`unimported_candidates`), in completion's order, each with the edit
//! completion's resolve writes (`import_edit`), kept where the import names something of the kind
//! the place wants, a type or a term, and offered where the document does not spell the name
//! anywhere else (`Spellings`, below).
//!
//! A member a selection `e.N` lacks is answered by the extension methods named `N` that an
//! import can name: those an object of the program holds (its own, inherited, exported; the
//! object reached by a path of packages and objects) and those at the top level of a package.
//! One is offered where the selection's own resolution would then find it: with its import
//! added to the file's for the question, the extensions of the nearest scope that has the name
//! (`lexical_extensions_for`) and the pick among them for the receiver (`pick_extension`) give
//! that method. And where the import cannot change what another use of the document means: no
//! identifier or operator of the document's text is spelled as a name the import brings
//! (`Spellings`: the lexer's tokens, whatever the build made of them), other than the names the
//! build's own reports say are missing, where they stand, since the import brings whatever its
//! owner holds under the name and would stand before or beside what such a use finds now. A use
//! that is not written (a `for` comprehension's `map`) is not seen. An object's import is written
//! as its wildcard (`import_forms`), which brings every name the object holds, where that holds
//! for each of them and the object has no `implicit` member; else as the method's own name,
//! where it holds for that name. The
//! receiver is read from the report's range, which starts where it starts: the longest expression
//! from there that the build typed (the index's node) and that the name follows, a dot, blanks or
//! line ends between.
//!
//! It all runs as completion does, in query mode, and leaves the session answering as it did.

use super::super::index::Files;
use super::super::resolve::{TermRef, TypeRef};
use super::super::unused::SelRef;
use super::super::{Env, ImportTarget, ResolvedImport, Worker};
use super::{decode, encode, import_segment, ImportEdit, CAP, ROOT_PKG};
use crate::ast::ListRef;
use crate::index::Lines;
use crate::intern::Name;
use crate::lsp::json::{obj, Json};
use crate::source::{FileId, ImportWanted, MissingKind, Span};
use crate::symbols::*;
use crate::tir::TExprId;
use crate::types::*;
use std::path::Path;

/// The names a document's text spells: its identifier and operator tokens as the lexer reads
/// them (none in a string or a comment; an interpolation's `$name` is one, and so is its
/// prefix, the interpolator's name; a backquoted one without its backquotes), each with its
/// span, and those among them that are a report's own:
/// the name the build says is missing, where the report has it.
struct Spellings {
    tokens: Vec<(Span, Box<str>)>,
    own: Vec<Span>,
}

impl Spellings {
    /// Of `text`, whose reports are `wanted`: a name not found is the token its report stands
    /// at; a member a selection lacks is the first token of that name after the selection's
    /// receiver (`receivers`, where it ends), or the first in the report's range where the
    /// receiver was not found.
    fn of(text: &str, wanted: &[ImportWanted], receivers: &[Option<(TExprId, u32)>]) -> Spellings {
        let interner = crate::intern::Interner::new();
        let lexed = crate::lexer::lex(text, &interner);
        let named = |t: &&crate::token::Token| matches!(t.kind, crate::token::Tok::Ident | crate::token::Tok::OpIdent | crate::token::Tok::InterpStart);
        let tokens: Vec<(Span, Box<str>)> = lexed.tokens.iter().filter(named).map(|t| (t.span, interner.get(t.name).into())).collect();
        let mut own = Vec::new();
        for (w, recv) in wanted.iter().zip(receivers) {
            let named = |(span, written): &&(Span, Box<str>)| **written == *w.name && w.span.start <= span.start && span.end <= w.span.end;
            match w.kind {
                MissingKind::Member => {
                    let from = recv.map_or(w.span.start, |(_, end)| end);
                    own.extend(tokens.iter().filter(named).find(|(span, _)| span.start >= from).map(|(span, _)| *span));
                }
                MissingKind::Term | MissingKind::Type => own.extend(tokens.iter().filter(named).map(|(span, _)| *span)),
            }
        }
        Spellings { tokens, own }
    }

    /// Whether the document spells one of `names` somewhere other than at a report's own name:
    /// a use the import of that name could change, resolved or not, or a definition of it. An
    /// operator is also spelled by its assignment form (`+=` for `+`), a prefix operator's
    /// method by the operator alone (`-` for `unary_-`), a setter by the name it sets (`level`
    /// for `level_=`, which `c.level = v` calls).
    fn hold(&self, names: &[&str]) -> bool {
        let operator = |n: &str| n.chars().next().is_some_and(|c| !(c.is_alphanumeric() || c == '_' || c == '$'));
        self.tokens.iter().any(|(span, written)| {
            let written: &str = written;
            !self.own.contains(span) && names.iter().any(|&n| n == written || n.strip_prefix("unary_") == Some(written) || n.strip_suffix("_=") == Some(written) || (operator(n) && written.strip_suffix('=') == Some(n)))
        })
    }
}

/// The form an extension's import is written in.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Form {
    /// `import a.O.*`: what the object holds.
    Wildcard,
    /// `import a.O.name`.
    ByName,
}

/// An object of the program that an import can name: its class, its path and its package's
/// path, their segments encoded as completion's are (none for the empty package).
struct NamedObject {
    class: ClassId,
    path: String,
    pkg: String,
}

/// How many objects that hold a not-found name are offered, the nearest to the document.
const NEAREST: usize = 12;

/// The names a document can use without writing them: an application's and an assignment's
/// methods, an extractor's, a `for` comprehension's. An object that holds one is not imported by
/// its wildcard, since such a use of the document's would not be seen.
const UNWRITTEN: [&str; 8] = ["apply", "update", "unapply", "unapplySeq", "map", "flatMap", "withFilter", "foreach"];

/// How near the package `pkg` of an object is to the document's package, the smaller the
/// nearer: the document's own package first, then the more leading segments the two share.
fn nearness(document: &str, pkg: &str) -> (bool, std::cmp::Reverse<usize>) {
    let shared = pkg.split('.').zip(document.split('.')).take_while(|(a, b)| a == b && !a.is_empty()).count();
    (pkg != document, std::cmp::Reverse(shared))
}

/// An owner of extension methods of one name that an import can name: the import's path, its
/// segments encoded as completion's are, what the import of the name from it is, and the methods.
struct ExtensionOwner {
    path: String,
    target: ImportTarget,
    methods: Vec<SymId>,
}

impl<'a> Worker<'a> {
    /// The answer to `import-actions` for the file `path`, whose reports that say something is
    /// missing are `wanted`, the imports asked for those marked so:
    /// `{"actions":[{"for":[<indices of wanted>],"title","edits":[{"range","newText"}]}]}`, an
    /// action an import, titled as the import it writes, with the reports it answers (one
    /// import that answers several is one action); null for a file the program has no text of,
    /// or a std one, which takes no import.
    pub fn import_actions(&mut self, path: &Path, wanted: &[ImportWanted], files: &Files) -> Json {
        let ids = files.ids_of(path);
        let Some(&first) = ids.first().filter(|&&f| !self.std_file(f)) else { return Json::Null };
        let sources: &'a crate::source::Sources = self.files;
        let text: &'a str = &sources[first.0 as usize].text;
        let mut actions: Vec<(ImportEdit, Vec<usize>)> = Vec::new();
        // The document's names as written, and where each report's own name stands: after its
        // receiver for a selection's, the report's place for a name not found.
        let receivers: Vec<Option<(TExprId, u32)>> = wanted.iter().map(|w| if w.kind == MissingKind::Member { self.receiver_in(&ids, text, w) } else { None }).collect();
        let spellings = Spellings::of(text, wanted, &receivers);
        // The program's objects an import can name, listed once for the command's reports.
        let top = Env { file: first, frames: Vec::new(), imports: Vec::new(), import_limit: None, visible_imports: None };
        let objects = if wanted.iter().any(|w| w.asked) { self.in_query(top, |me| me.nameable_objects()) } else { Vec::new() };
        for (i, w) in wanted.iter().enumerate().filter(|(_, w)| w.asked) {
            let found = match (w.kind, receivers[i]) {
                (MissingKind::Member, Some((recv, _))) => self.member_imports(&ids, first, text, w, recv, &spellings, &objects),
                (MissingKind::Member, None) => Vec::new(),
                (MissingKind::Term | MissingKind::Type, _) => self.name_imports(&ids, first, text, w, &spellings, &objects),
            };
            for edit in found {
                match actions.iter_mut().find(|(e, _)| e.import == edit.import && (e.start, e.end) == (edit.start, edit.end) && e.text == edit.text) {
                    Some((_, of)) if of.contains(&i) => {}
                    Some((_, of)) => of.push(i),
                    None => actions.push((edit, vec![i])),
                }
            }
        }
        let lines = Lines::new(text);
        let actions = actions
            .into_iter()
            .map(|(edit, of)| {
                let range = lines.range(Span::new(edit.start, edit.end), files.positions);
                let edits = Json::Arr(vec![obj([("range", range), ("newText", edit.text.into())])]);
                obj([("for", Json::Arr(of.into_iter().map(|i| (i as u32).into()).collect())), ("title", edit.import.into()), ("edits", edits)])
            })
            .collect();
        obj([("actions", Json::Arr(actions))])
    }

    /// The imports that bring the name `w` says is not found where it stands, none where the
    /// document spells the name elsewhere (`Spellings::hold`): completion's
    /// candidates out of the scope for the whole spelling (its floor of two characters theirs),
    /// in its order, each with its edit, where the import resolves and names a type for a type's
    /// place, a term or a class (which its creator application makes a term) for a term's.
    fn name_imports(&mut self, ids: &[FileId], first: FileId, text: &str, w: &ImportWanted, spellings: &Spellings, objects: &[NamedObject]) -> Vec<ImportEdit> {
        let offset = w.span.start;
        let file = self.file_at(ids, offset);
        let lower = w.name.to_lowercase();
        // The import is by name: none where the document spells the name anywhere else, a use the
        // import could change (another tree of the file that finds the name its own way).
        if self.std_file(file) || spellings.hold(&[&w.name]) {
            return Vec::new();
        }
        let types = w.kind == MissingKind::Type;
        let mut out: Vec<ImportEdit> = Vec::new();
        // Completion's candidates, where completion offers any: a name of two characters or more.
        if lower.chars().count() >= 2 {
            let binders = self.binders(file);
            let catalogue = self.catalogue();
            let program = self.program_entries();
            let base = Env { file, frames: Vec::new(), imports: Vec::new(), import_limit: None, visible_imports: None };
            let paths: Vec<String> = self.in_query(base, |me| {
                me.enter_scope_at(file, offset, &binders);
                let mut more = Vec::new();
                me.unimported_candidates(types, &lower, &w.name, offset, &[&program, &catalogue.external], &mut more);
                more.into_iter().filter(|c| *c.name == *w.name).filter_map(|c| c.import.map(|(path, _)| path)).collect()
            });
            if let Some(ix) = self.index.as_mut() {
                ix.catalogue = Some(catalogue);
            }
            self.keep_imports(binders);
            for path in paths {
                let Some(edit) = self.import_edit(file, first, text, offset, &path) else { continue };
                let named = if types { edit.ty.is_some() } else { edit.term.is_some() || matches!(edit.ty, Some(TypeRef::Class(_))) };
                // A given or an implicit by name would join every implicit search of the document.
                if named && !self.brings_implicit(edit.ty, edit.term) && !out.iter().any(|e| e.import == edit.import) {
                    out.push(edit);
                }
            }
        }
        self.object_member_imports(file, first, text, w, spellings, objects, &mut out);
        out
    }

    /// After completion's candidates for the name `w` says is not found (`out`), the imports of
    /// the objects of the program that hold a member of the name of the kind the place wants: a
    /// term (a def, a val, an object, an enum's case; inherited and exported ones) or, for a
    /// type's place, a class or an alias. What holds one is what an import's own resolution
    /// finds (`module_term`, `module_type`): a companion's members the compiler made (`apply`,
    /// `unapply`, `fromProduct`, an enum's `values` and `ordinal`) are not in the table it reads,
    /// so their import would not resolve; what the table holds unwritten (a case class's
    /// companion's `toString`) is no holder's (`member_written`). The
    /// nearest objects first (the document's own package, then the longer package prefix shared
    /// with it, then the path), the first `NEAREST` whose import resolves; the object's wildcard
    /// where the document allows it (`import_forms`) and the name then resolves at its place to
    /// that member, else the member's own import, completion's edit for its path. An object whose
    /// member completion already offers by that import adds no action.
    #[allow(clippy::too_many_arguments)]
    fn object_member_imports(&mut self, file: FileId, first: FileId, text: &str, w: &ImportWanted, spellings: &Spellings, objects: &[NamedObject], out: &mut Vec<ImportEdit>) {
        let offset = w.span.start;
        let types = w.kind == MissingKind::Type;
        let name = self.interner.intern(&w.name);
        // A name no program writes is no candidate: a `derives` clause's `derived$T`, as for
        // completion, and a setter's (`x_=`), which a `var` makes.
        if !super::importable(&w.name) || w.name.ends_with("_=") {
            return;
        }
        let top = || Env { file, frames: Vec::new(), imports: Vec::new(), import_limit: None, visible_imports: None };
        let (document, mut holders): (String, Vec<&NamedObject>) = self.in_query(top(), |me| {
            let p = me.pkg_chain()[0];
            (me.package_key(p), objects.iter().filter(|o| me.holds_member(o.class, name, types)).collect())
        });
        if holders.is_empty() {
            return;
        }
        holders.sort_by(|a, b| (nearness(&document, &a.pkg), &a.path).cmp(&(nearness(&document, &b.pkg), &b.path)));
        let last = encode(&w.name);
        let selector = import_segment(&w.name);
        let binders = self.binders(file);
        let mut taken = 0;
        for o in holders {
            if taken == NEAREST {
                break;
            }
            let segments: Vec<String> = o.path.split('.').map(decode).chain(std::iter::once(w.name.to_string())).collect();
            // By name: completion's own edit for the path, or, for an object of the empty package,
            // which that edit cannot name, the path as it is where the file sees the object.
            let by_name = if o.pkg.is_empty() {
                let resolves = self.in_query(top(), |me| {
                    me.enter_scope_at(file, offset, &binders);
                    me.resolves_with(ResolvedImport { name: Some(name), target: ImportTarget::ClassMember(o.class, name), hidden: ListRef::EMPTY, bound: None, depth: 0, stmt: u32::MAX, unimports_predef: None, sel: SelRef::NONE }, o.class, name, types, w.span)
                });
                let reached = resolves && self.in_query(top(), |me| me.member_reached(o.class, name, types));
                self.written_import(file, &segments, false).filter(|_| reached).map(|import| self.placed_edit(file, first, text, offset, ImportTarget::ClassMember(o.class, name), false, import, &selector))
            } else {
                self.import_edit(file, first, text, offset, &format!("{}.{}", o.path, last)).filter(|edit| if types { edit.ty.is_some() } else { edit.term.is_some() || matches!(edit.ty, Some(TypeRef::Class(_))) })
            };
            let Some(by_name) = by_name else { continue };
            if out.iter().any(|e| e.import == by_name.import) {
                continue;
            }
            let wildcard = self.import_forms(Some(o.class), spellings, true, file).contains(&Form::Wildcard)
                && self.in_query(top(), |me| {
                    me.enter_scope_at(file, offset, &binders);
                    me.resolves_with(ResolvedImport { name: None, target: ImportTarget::ClassAll(o.class), hidden: ListRef::EMPTY, bound: None, depth: 0, stmt: u32::MAX, unimports_predef: None, sel: SelRef::NONE }, o.class, name, types, w.span)
                });
            let edit = match wildcard.then(|| self.written_import(file, &segments, true)).flatten() {
                Some(import) => self.placed_edit(file, first, text, offset, ImportTarget::ClassMember(o.class, name), true, import, &selector),
                None => by_name,
            };
            if !out.iter().any(|e| e.import == edit.import) {
                out.push(edit);
                taken += 1;
            }
        }
        self.keep_imports(binders);
    }

    /// The edit of the import `import` of `target` for the site at `offset`: on a line of its
    /// own, or its selector (`selector`, the object's `*` with `wildcard`) in a clause of its
    /// prefix (`import_placed`).
    #[allow(clippy::too_many_arguments)]
    fn placed_edit(&mut self, file: FileId, first: FileId, text: &str, offset: u32, target: ImportTarget, wildcard: bool, import: String, selector: &str) -> ImportEdit {
        let placed = self.import_placed(file, first, text, offset, target, wildcard);
        let written = placed.text(&import, if wildcard { "*" } else { selector });
        ImportEdit { start: placed.start, end: placed.end, text: written, import, ty: None, term: None }
    }

    /// Whether the object `c` holds a member named `name` that an import of it brings for the
    /// kind of place: a type for a type's, a term or a class for a term's; no given and no
    /// implicit (`brings_implicit`).
    fn holds_member(&mut self, c: ClassId, name: Name, types: bool) -> bool {
        let ty = self.quietly(|me| me.module_type(c, name));
        let term = self.quietly(|me| me.module_term(c, name));
        if self.brings_implicit(ty, term) {
            return false;
        }
        if types {
            return ty.is_some();
        }
        matches!(ty, Some(TypeRef::Class(_))) || term.is_some_and(|r| self.member_written(c, r))
    }

    /// Whether an import by name of what a name is as a type and as a term would bring a given
    /// or an implicit (a value, a method, one of an overloaded name's, an implicit class): it
    /// would join every implicit search of the document, which may then find another instance
    /// or an ambiguity. Such a name gets no import from a quick fix.
    fn brings_implicit(&self, ty: Option<TypeRef>, term: Option<TermRef>) -> bool {
        let implicit_class = |k: ClassId| self.syms.class(k).mods & crate::ast::mods::IMPLICIT != 0;
        let implicit_term = |r: TermRef| match r {
            TermRef::Class(k) | TermRef::ValueClass(_, k) => implicit_class(k),
            other => other.sym().is_some_and(|s| self.syms.is_given(s) || self.syms.alternatives(s).is_some_and(|alts| alts.iter().any(|&a| self.syms.is_given(a)))),
        };
        matches!(ty, Some(TypeRef::Class(k)) if implicit_class(k)) || term.is_some_and(implicit_term)
    }

    /// Whether the term `r` the object `c` holds is one a program wrote, not a member the
    /// compiler made in the object: one it inherits (a parent's, a library's, a class
    /// parameter's), or its own with its definition's tree, or an object, an enum's case, an
    /// overloaded name or an export's forwarder, which have none by their kind. What is left is
    /// the object's own member without a tree: a case class's companion's `toString`.
    fn member_written(&self, c: ClassId, r: TermRef) -> bool {
        let Some(s) = r.sym() else { return true };
        let info = self.syms.sym(s);
        info.def.is_some() || info.owner != Owner::Class(c) || matches!(info.kind, SymKind::Object(_) | SymKind::Overloaded(_) | SymKind::EnumValue(_)) || self.syms.forward_of(s).is_some()
    }

    /// Whether the member `name` of the object `c` that `holds_member` found is one an import
    /// reaches from outside the object: no private or protected one (`import_target`'s test).
    fn member_reached(&mut self, c: ClassId, name: Name, types: bool) -> bool {
        let hidden = crate::ast::mods::PRIVATE | crate::ast::mods::PROTECTED;
        let ty = self.quietly(|me| me.module_type(c, name));
        let public_type = match ty {
            Some(TypeRef::Class(k)) => self.syms.class(k).mods & hidden == 0,
            Some(TypeRef::Alias(a)) => {
                let (file, def) = (self.syms.aliases[a.idx()].file, self.syms.aliases[a.idx()].def);
                def.map_or(true, |d| self.ast(file).def(d).mods & hidden == 0)
            }
            _ => false,
        };
        if types {
            return public_type;
        }
        let term = self.quietly(|me| me.module_term(c, name));
        term.is_some_and(|r| self.public_term(r)) || (term.is_none() && public_type && matches!(ty, Some(TypeRef::Class(_))))
    }

    /// Whether, with the import `imp` added to the environment's file's as `picked_with` adds
    /// one, `name` resolves at `at` without a report to the member of the object `c`: the
    /// typer's own lookup, as a term or, for a type's place, as a type.
    fn resolves_with(&mut self, imp: ResolvedImport, c: ClassId, name: Name, types: bool, at: Span) -> bool {
        let f = self.env.file.0 as usize;
        self.import_count();
        let saved = self.file_imports[f].clone();
        let mut imports: Vec<ResolvedImport> = saved.as_deref().cloned().unwrap_or_default();
        let place = if imp.name.is_some() { 0 } else { imports.iter().take_while(|i| i.name.is_some()).count() };
        imports.insert(place, imp);
        self.file_imports[f] = Some(std::sync::Arc::new(imports));
        let key = |r: TermRef| match r {
            TermRef::Class(k) | TermRef::ValueClass(_, k) => (0, k.0),
            TermRef::Package(p) => (1, p.0),
            other => (2, other.sym().map_or(u32::MAX, |s| s.0)),
        };
        let as_type = |me: &mut Self| {
            let wanted = me.quietly(|me| me.module_type(c, name));
            wanted.is_some() && me.quietly(|me| me.lookup_type_at(name, at)) == wanted
        };
        let resolved = if types {
            as_type(self)
        } else {
            match self.quietly(|me| me.module_term(c, name)).map(key) {
                Some(wanted) => self.quietly(|me| me.lookup_term_at(name, at)).map(key) == Some(wanted),
                // A class alone, which its creator application makes a term.
                None => as_type(self),
            }
        };
        self.file_imports[f] = saved;
        resolved
    }

    /// The import of the path `segments` as it is written at the top of `file`, with the
    /// object's wildcard in the place of its last name where `wildcard`: every segment as an
    /// import's path spells it; from `_root_` where the path starts at a top-level package whose
    /// name is something else there; `None` where it starts at an object of the empty package
    /// that the file's top does not see, which only the empty package's files do.
    fn written_import(&mut self, file: FileId, segments: &[String], wildcard: bool) -> Option<String> {
        let head = self.interner.intern(segments.first()?);
        let top = Env { file, frames: Vec::new(), imports: Vec::new(), import_limit: None, visible_imports: None };
        let rooted = self.in_query(top, |me| match me.quietly(|me| me.pkg_term(ROOT_PKG, head))? {
            found @ TermRef::Package(_) => Some(me.import_rooted(head, found)),
            found => {
                let here = me.quietly(|me| me.lookup_term_at(head, Span::new(0, 0)))?;
                let object = |r: TermRef| me.path_object(r);
                (object(here).is_some() && object(here) == object(found)).then_some(false)
            }
        })?;
        let mut written: Vec<String> = segments.iter().map(|s| import_segment(s)).collect();
        if wildcard {
            written.pop();
            written.push("*".to_string());
        }
        Some(format!("import {}{}", if rooted { "_root_." } else { "" }, written.join(".")))
    }

    /// The imports that bring an extension method for the selection `w` reports a member
    /// missing of: by name, one an owner whose method the selection would find with it, in the
    /// order of the owners' paths.
    #[allow(clippy::too_many_arguments)]
    fn member_imports(&mut self, ids: &[FileId], first: FileId, text: &str, w: &ImportWanted, recv: TExprId, spellings: &Spellings, objects: &[NamedObject]) -> Vec<ImportEdit> {
        let offset = w.span.start;
        let file = self.file_at(ids, offset);
        if self.std_file(file) {
            return Vec::new();
        }
        // The receiver's type, read while the index is in place.
        let Some(recv_ty) = self.prog.type_of(recv) else { return Vec::new() };
        let name = self.interner.intern(&w.name);
        let top = || Env { file, frames: Vec::new(), imports: Vec::new(), import_limit: None, visible_imports: None };
        let owners = self.in_query(top(), |me| me.extension_owners(name, objects));
        if owners.is_empty() {
            return Vec::new();
        }
        // The forms the document allows each owner's import, then the first of them with which the
        // selection finds the owner's method.
        let by_name = !spellings.hold(&[&w.name]);
        let allowed: Vec<Vec<Form>> = owners
            .iter()
            .map(|o| {
                let object = match o.target {
                    ImportTarget::ClassMember(c, _) => Some(c),
                    _ => None,
                };
                self.import_forms(object, spellings, by_name, file)
            })
            .collect();
        if allowed.iter().all(Vec::is_empty) {
            return Vec::new();
        }
        let binders = self.binders(file);
        let chosen: Vec<Option<Form>> = self.in_query(top(), |me| {
            me.enter_scope_at(file, offset, &binders);
            let recv_ty = me.zonk(recv_ty);
            let with = |me: &mut Self, o: &ExtensionOwner, form: Form| {
                let (name, target) = match (form, o.target) {
                    (Form::Wildcard, ImportTarget::ClassMember(c, _)) => (None, ImportTarget::ClassAll(c)),
                    _ => (Some(name), o.target),
                };
                me.picked_with(ResolvedImport { name, target, hidden: ListRef::EMPTY, bound: None, depth: 0, stmt: u32::MAX, unimports_predef: None, sel: SelRef::NONE }, &o.methods, w, recv, recv_ty)
            };
            owners.iter().zip(&allowed).map(|(o, forms)| forms.iter().copied().find(|&form| with(me, o, form))).collect()
        });
        self.keep_imports(binders);
        let selector = import_segment(&w.name);
        let mut out = Vec::new();
        for (o, form) in owners.iter().zip(chosen) {
            let Some(form) = form else { continue };
            let segments: Vec<String> = o.path.split('.').map(decode).collect();
            let Some(import) = self.written_import(file, &segments, form == Form::Wildcard) else { continue };
            out.push(self.placed_edit(file, first, text, offset, o.target, form == Form::Wildcard, import, &selector));
        }
        out
    }

    /// The forms the import of an extension from the owner `o` may take in the document, the one
    /// to write first: the object's wildcard, which also brings what else of the object the
    /// document goes on to use, where the document spells no name the object holds
    /// (`module_names`, its extensions among them; `Spellings::hold`) and the object has no
    /// `implicit` member, which a wildcard would bring into every implicit search of the file;
    /// then the method's own name, where the document does not spell it (`by_name`). A
    /// package's top-level extension is imported by name alone (no `object`). An object that
    /// holds a name a document uses without writing it (`UNWRITTEN`, an interpolator) is not
    /// imported by its wildcard either.
    fn import_forms(&mut self, object: Option<ClassId>, spellings: &Spellings, by_name: bool, file: FileId) -> Vec<Form> {
        let mut forms = Vec::new();
        if let (true, Some(c)) = (by_name, object) {
            let top = Env { file, frames: Vec::new(), imports: Vec::new(), import_limit: None, visible_imports: None };
            let (names, closed) = self.in_query(top, |me| {
                let mut names = me.module_names(c, false);
                names.extend(me.module_names(c, true));
                // And the extension methods it exports, which its wildcard brings as its own.
                if let Some(exports) = me.exports_of(c) {
                    names.extend(exports.extensions.keys().copied());
                }
                (names, me.holds_implicit(c) || me.holds_interpolator(c))
            });
            let names: Vec<&str> = names.iter().map(|&n| self.name_ref(n)).collect();
            if !closed && !names.iter().any(|n| UNWRITTEN.contains(n)) && !spellings.hold(&names) {
                forms.push(Form::Wildcard);
            }
        }
        if by_name {
            forms.push(Form::ByName);
        }
        forms
    }

    /// Whether the object `c` holds an interpolator, its own or inherited: an extension method of
    /// a `StringContext`, which a document uses as a string's prefix, not as a name.
    fn holds_interpolator(&mut self, c: ClassId) -> bool {
        let Some(context) = self.b.string_context else { return false };
        self.complete_class(c);
        let bases: Vec<ClassId> = self.syms.class(c).base_types.iter().map(|&(b, _)| b).collect();
        bases.into_iter().any(|b| {
            self.complete_class(b);
            let extensions: Vec<SymId> = self.syms.class_raw(b).extensions.clone();
            extensions.into_iter().any(|s| {
                let leading = self.syms.sym(s).ext_clauses as usize;
                let receiver = self.sig_of(s).clauses.iter().take(leading).find(|cl| !cl.is_using).and_then(|cl| cl.params.first().map(|p| p.ty));
                receiver.is_some_and(|t| self.class_of(t) == Some(context))
            })
        })
    }

    /// Whether the object `c` holds an `implicit` member, its own or inherited: a value, a
    /// method or a class.
    fn holds_implicit(&mut self, c: ClassId) -> bool {
        self.complete_class(c);
        let bases: Vec<ClassId> = self.syms.class(c).base_types.iter().map(|&(b, _)| b).collect();
        bases.into_iter().any(|b| {
            self.complete_class(b);
            let info = self.syms.class(b);
            info.members.values().any(|&m| self.syms.sym(m).mods & crate::ast::mods::IMPLICIT != 0) || info.nested.values().any(|&n| self.syms.class(n).mods & crate::ast::mods::IMPLICIT != 0)
        })
    }

    /// Whether, with the import `imp` added to the environment's file's (a named one first, a
    /// wildcard after the named ones, as the file's own are listed), the selection of the name
    /// `w` reports on the receiver `recv` of type `recv_ty` finds one of `methods`: the typer's
    /// own lexical step for an extension method. The receiver the pick may widen or convert is a
    /// node of the question's own.
    fn picked_with(&mut self, imp: ResolvedImport, methods: &[SymId], w: &ImportWanted, recv: TExprId, recv_ty: TypeId) -> bool {
        let name = self.interner.intern(&w.name);
        let f = self.env.file.0 as usize;
        self.import_count();
        let saved = self.file_imports[f].clone();
        let mut imports: Vec<ResolvedImport> = saved.as_deref().cloned().unwrap_or_default();
        let at = if imp.name.is_some() { 0 } else { imports.iter().take_while(|i| i.name.is_some()).count() };
        imports.insert(at, imp);
        self.file_imports[f] = Some(std::sync::Arc::new(imports));
        let probe = self.prog.add(self.prog.expr(recv));
        self.prog.set_type(probe, recv_ty);
        let mark = self.snapshot();
        let picked = self.muted(|me| {
            let (lexical, _) = me.lexical_extensions_for(name, recv_ty);
            me.pick_extension(lexical, probe, recv_ty, &mut None, false, None)
        });
        self.rollback(&mark);
        self.file_imports[f] = saved;
        picked.is_some_and(|call| methods.contains(&call.sym))
    }

    /// The objects of the program that an import can name, by their paths: each package's (the
    /// root's are the empty package's, named without a path), a class's companion among them
    /// (the entry's object beside its class), and the objects nested in them, which are their
    /// members; private ones left out.
    fn nameable_objects(&mut self) -> Vec<NamedObject> {
        let mut out = Vec::new();
        for i in 0..self.syms.pkgs.len() {
            let p = PkgId(i as u32);
            let pkg = self.package_key(p);
            let objects: Vec<ClassId> = self
                .syms
                .members(p)
                .flat_map(|(_, e)| {
                    // A package of the same name takes an import's path: its object is not named.
                    if e.pkg.is_some() {
                        return [None, None];
                    }
                    let object = e.term.and_then(|s| match self.syms.sym(s).kind {
                        SymKind::Object(c) => Some(c),
                        _ => None,
                    });
                    [e.class, object]
                })
                .flatten()
                .collect();
            for c in objects {
                self.nameable_object(c, &pkg, &pkg, &mut out);
            }
        }
        out.sort_by(|a, b| a.path.cmp(&b.path));
        out.dedup_by(|later, kept| later.path == kept.path);
        out
    }

    /// The path of the package `p` with its segments encoded as an object's path has them, empty
    /// for the root.
    fn package_key(&self, p: PkgId) -> String {
        if p == ROOT_PKG {
            return String::new();
        }
        self.pkg_path(p).split('.').map(encode).collect::<Vec<_>>().join(".")
    }

    /// `nameable_objects` of the program object `c` at the path `owner` of the package `pkg`, and
    /// of the objects nested in it.
    fn nameable_object(&mut self, c: ClassId, owner: &str, pkg: &str, out: &mut Vec<NamedObject>) {
        let (file, mods, kind, own) = {
            let info = self.syms.class(c);
            (info.file, info.mods, info.kind, info.name)
        };
        let own = self.name_str(own);
        if kind != ClassKind::Object || !self.program_source(file) || mods & (crate::ast::mods::PRIVATE | crate::ast::mods::PROTECTED) != 0 || !super::importable(&own) || out.len() >= 1 << 20 {
            return;
        }
        let path = if owner.is_empty() { encode(&own) } else { format!("{}.{}", owner, encode(&own)) };
        out.push(NamedObject { class: c, path: path.clone(), pkg: pkg.to_string() });
        // An object nested in an object is its member.
        let nested: Vec<ClassId> = self.syms.class(c).members.values().filter_map(|&m| match self.syms.sym(m).kind {
            SymKind::Object(n) => Some(n),
            _ => None,
        }).collect();
        for n in nested {
            self.nameable_object(n, &path, pkg, out);
        }
    }

    /// The owners of the program's extension methods named `name` that an import can name, by
    /// their paths: each package's own at its top level, and each of `objects` (its own,
    /// inherited and exported ones), as many as completion lists.
    fn extension_owners(&mut self, name: Name, objects: &[NamedObject]) -> Vec<ExtensionOwner> {
        let mut out = Vec::new();
        let last = encode(self.name_ref(name));
        // The root's top-level extensions are in scope wherever its objects can be named.
        for i in 1..self.syms.pkgs.len() {
            let p = PkgId(i as u32);
            let methods: Vec<SymId> = self.syms.package_object_entry(p, name).map_or_else(Vec::new, |e| e.extensions.clone());
            let methods = self.importable_extensions(methods);
            if !methods.is_empty() {
                out.push(ExtensionOwner { path: format!("{}.{}", self.pkg_path(p), last), target: ImportTarget::PkgMember(p, name), methods });
            }
        }
        for o in objects {
            self.complete_class(o.class);
            let mut methods = Vec::new();
            self.module_extensions(o.class, name, &mut methods);
            let methods = self.importable_extensions(methods);
            if !methods.is_empty() {
                out.push(ExtensionOwner { path: format!("{}.{}", o.path, last), target: ImportTarget::ClassMember(o.class, name), methods });
            }
        }
        out.sort_by(|a, b| a.path.cmp(&b.path));
        out.dedup_by(|later, kept| later.path == kept.path);
        out.truncate(CAP);
        out
    }

    /// Those of `methods` that are the program's and that an import reaches: no private or
    /// protected one.
    fn importable_extensions(&self, mut methods: Vec<SymId>) -> Vec<SymId> {
        methods.retain(|&s| {
            let info = self.syms.sym(s);
            self.program_source(info.file) && info.mods & (crate::ast::mods::PRIVATE | crate::ast::mods::PROTECTED) == 0
        });
        methods.sort_by_key(|s| s.0);
        methods.dedup();
        methods
    }

    /// The node of the receiver of the selection `w` reports, and where its text ends: the report's range starts where the
    /// receiver starts, and the receiver is the longest text from there that the name follows
    /// inside the range (a dot, blanks or line ends between; the name as it is, not between
    /// backquotes) and that the build typed as an expression (the index's node of that span, or
    /// of what its parentheses hold).
    fn receiver_in(&self, ids: &[FileId], text: &str, w: &ImportWanted) -> Option<(TExprId, u32)> {
        let (start, end) = (w.span.start as usize, (w.span.end as usize).min(text.len()));
        let range = text.get(start..end)?;
        let word = |c: char| c.is_alphanumeric() || c == '_' || c == '$';
        let symbolic = !w.name.chars().next().is_some_and(word);
        let part = |c: char| if symbolic { !(word(c) || c.is_whitespace() || "()[]{}.,;`'\"".contains(c)) } else { word(c) };
        let mut best: Option<(usize, TExprId)> = None;
        let form: &str = &w.name;
        let mut from = 0;
        while let Some(i) = range.get(from..).and_then(|rest| rest.find(form)).map(|i| i + from) {
            from = i + form.chars().next().map_or(1, char::len_utf8);
            let after = i + form.len();
            let whole = !range[..i].chars().next_back().is_some_and(part) && !range[after..].chars().next().is_some_and(part);
            let head = range[..i].trim_end();
            let head = head.strip_suffix('.').map_or(head, str::trim_end);
            if !whole || head.is_empty() || best.is_some_and(|(len, _)| len >= head.len()) {
                continue;
            }
            // The expression itself, or the one its parentheses hold.
            let mut inner = head;
            let mut at = start;
            loop {
                if let Some(te) = self.index_node_at(ids, Span::new(at as u32, (at + inner.len()) as u32)) {
                    best = Some((head.len(), te));
                    break;
                }
                let Some(held) = inner.strip_prefix('(').and_then(|r| r.strip_suffix(')')) else { break };
                let skipped = held.len() - held.trim_start().len();
                at += 1 + skipped;
                inner = held.trim();
            }
        }
        best.map(|(len, te)| (te, (start + len) as u32))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn report(text: &str, at: &str, name: &str, kind: MissingKind) -> ImportWanted {
        let start = text.find(at).expect("the report's text") as u32;
        ImportWanted { name: name.into(), kind, span: Span::new(start, start + at.len() as u32), asked: true }
    }

    /// What a document spells is its identifier and operator tokens: a comment and a string
    /// spell nothing, an interpolation's name and a backquoted name do; an operator is spelled
    /// by its assignment form too, a prefix operator's method by the operator alone.
    #[test]
    fn a_document_spells_its_tokens() {
        let none = Spellings::of("// asGlyph, said\nval s = \"asGlyph\" /* asGlyph */\n", &[], &[]);
        assert!(!none.hold(&["asGlyph"]));
        assert!(Spellings::of("val name = 1\nval t = s\"$name and ${other}\"\n", &[], &[]).hold(&["other"]));
        assert!(Spellings::of("val t = s\"$name\"\n", &[], &[]).hold(&["name"]));
        assert!(Spellings::of("val x = `asGlyph`\n", &[], &[]).hold(&["asGlyph"]));
        // An interpolator's prefix is a name the document writes; a setter is written as the name it sets.
        let prefixed = Spellings::of("val t = tag\"x\" + raw\"\"\"y\"\"\"\n", &[], &[]);
        assert!(prefixed.hold(&["tag"]) && prefixed.hold(&["raw"]) && !prefixed.hold(&["x"]) && !prefixed.hold(&["y"]));
        let set = Spellings::of("def g(c: Cell): Unit = c.level = 5\n", &[], &[]);
        assert!(set.hold(&["level_="]) && !set.hold(&["leve_="]));
        let ops = Spellings::of("def f(c: C) = { var d = c; d += 1; -d }\n", &[], &[]);
        assert!(ops.hold(&["+"]) && ops.hold(&["unary_-"]) && !ops.hold(&["unary_!"]) && !ops.hold(&["*"]));
        // A word's assignment-like neighbour is no spelling of it, nor is a longer name.
        assert!(!Spellings::of("val asGlyphs = 1\n", &[], &[]).hold(&["asGlyph"]));
    }

    /// The objects that hold a name are offered the nearest first: the document's own package,
    /// then the longer package prefix shared with it; the empty package shares nothing.
    #[test]
    fn the_nearest_objects_come_first() {
        let mut pkgs = vec!["zeta", "ord.a", "ord", "ord.a.b.c", "ord.a.b", "", "ord.x.b"];
        pkgs.sort_by_key(|p| (nearness("ord.a.b", p), *p));
        assert_eq!(pkgs, ["ord.a.b", "ord.a.b.c", "ord.a", "ord", "ord.x.b", "", "zeta"]);
        assert!(nearness("", "") < nearness("", "a"));
        assert_eq!(nearness("a.b", "x.a.b"), nearness("a.b", "zeta"));
    }

    /// The names the build's own reports say are missing are set apart where they stand: a name
    /// not found at its report; a missing member as the first token of the name after its
    /// receiver, or the first in the report's range where the receiver was not found. Any other
    /// token of the name holds.
    #[test]
    fn a_reports_own_name_is_set_apart() {
        let text = "val p = plain(1)\nval q = plain(2)\n";
        let first = report(text, "plain", "plain", MissingKind::Term);
        assert!(Spellings::of(text, &[first.clone()], &[None]).hold(&["plain"]));
        let second = ImportWanted { span: Span::new(first.span.start + 17, first.span.end + 17), ..first.clone() };
        assert!(!Spellings::of(text, &[first, second], &[None, None]).hold(&["plain"]));
        // The receiver ends in the name, as a member that resolved: that token holds.
        let text = "val e = box.asGlyph.asGlyph\n";
        let outer = report(text, "box.asGlyph.asGlyph", "asGlyph", MissingKind::Member);
        let after = text.find(".asGlyph\n").expect("the outer selection") as u32;
        assert!(Spellings::of(text, &[outer], &[Some((TExprId(0), after))]).hold(&["asGlyph"]));
        // No receiver found: the first token of the name in the range is the report's.
        let text = "val c = n plus 1\n";
        let infix = report(text, "n plus 1", "plus", MissingKind::Member);
        assert!(!Spellings::of(text, &[infix], &[None]).hold(&["plus"]));
    }
}
