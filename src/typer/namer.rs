use super::Worker;
use crate::ast::{self, mods, DefId, DefKind, Stmt};
use crate::names;
use crate::source::FileId;
use crate::symbols::*;
use crate::types::*;
use std::sync::Arc;

impl<'a> Worker<'a> {
    /// Enters the definitions of every file: the std files parsed with the program first, in
    /// their slot order, then the program's files. A std file entered later goes through
    /// `enter_file_defs` from `stdlib.rs`.
    pub fn enter_all(&mut self) {
        self.enter_parsed_std();
        for i in 0..self.asts.len() {
            let file = FileId(i as u32);
            if self.std.slot_of(file).is_some() {
                continue;
            }
            self.enter_file_defs(file);
        }
    }

    pub(super) fn enter_file_defs(&mut self, file: FileId) {
        let ast = self.ast(file);
        let mut pkg = ROOT_PKG;
        for &n in &ast.package {
            pkg = self.syms.sub_pkg(pkg, n);
        }
        self.file_pkgs[file.0 as usize] = pkg;
        if !ast.top_exports.is_empty() {
            self.syms.pkgs[pkg.idx()].export_files.push(file);
        }
        self.env.file = file;
        for &d in &ast.top_level {
            self.enter_def(file, Owner::Package(pkg), d);
        }
        if self.forked {
            self.file_pkgs.publish(file.0 as usize);
            self.file_opaques.publish(file.0 as usize);
        }
    }

    /// Whether a top-level definition `new` of a package meets one of the other side, the std's
    /// against the program's: the program's is what the name means. `Some(true)` when `new` is
    /// the std's and gives way, `Some(false)` when `old` is the std's and gives way.
    fn shadowing(&self, old_file: FileId, new_file: FileId) -> Option<bool> {
        let (old_std, new_std) = (self.source(old_file).is_std, self.source(new_file).is_std);
        match (old_std, new_std) {
            (false, true) => Some(true),
            (true, false) => Some(false),
            _ => None,
        }
    }

    /// Registers a term of its owner; false when a definition of the program shadows this one
    /// of the std, which then stays out of the tables.
    pub(super) fn register_term(&mut self, owner: Owner, sym: SymId) -> bool {
        let (name, span, file) = {
            let s = self.syms.sym(sym);
            (s.name, s.span, s.file)
        };
        let old = match owner {
            Owner::Package(p) => self.syms.pkg(p).entries.get(&name).and_then(|e| e.term),
            Owner::Class(c) => self.syms.class(c).members.get(&name).copied(),
            Owner::Local => None,
        };
        if let (Owner::Package(p), Some(o)) = (owner, old) {
            match self.shadowing(self.syms.sym(o).file, file) {
                Some(true) => {
                    self.note_shadowed(owner, name, file);
                    if let SymKind::Object(c) = self.syms.sym(sym).kind {
                        self.syms.class_mut(c).replaced = true;
                        self.std_mut().shadowed_classes.insert(c, ());
                    }
                    return false;
                }
                Some(false) => {
                    let old_file = self.syms.sym(o).file;
                    self.note_shadowed(owner, name, old_file);
                    self.retire_std_term(p, o);
                }
                None => {}
            }
        }
        // A std object whose name a program class has stays out with it: the program's class
        // and object of one name are one pair, and the std's object would be the class's
        // companion otherwise.
        if let (Owner::Package(p), None, true) = (owner, old, self.source(file).is_std) {
            let program_type = {
                let e = self.syms.pkg(p).entries.get(&name);
                e.and_then(|e| e.class).map(|c| self.syms.class(c).file).or_else(|| e.and_then(|e| e.alias).map(|a| self.syms.aliases[a.idx()].file))
            };
            if program_type.map_or(false, |f| !self.source(f).is_std) {
                self.note_shadowed(owner, name, file);
                if let SymKind::Object(c) = self.syms.sym(sym).kind {
                    self.syms.class_mut(c).replaced = true;
                    self.std_mut().shadowed_classes.insert(c, ());
                }
                return false;
            }
        }
        let old = match owner {
            Owner::Package(p) => self.syms.pkg(p).entries.get(&name).and_then(|e| e.term),
            _ => old,
        };
        if let Some(old) = old.filter(|&o| self.overloads(o, sym)) {
            self.restrict_overload(sym);
            self.add_alternative(owner, old, sym);
            return true;
        }
        let duplicate = match owner {
            Owner::Package(p) => {
                let replaced = old.and_then(|o| match (self.syms.sym(o).kind, self.syms.sym(sym).kind) {
                    (SymKind::Object(a), SymKind::Object(b)) => self.java_stand_in_of(a, b),
                    _ => None,
                });
                if let Some(c) = replaced {
                    self.syms.class_mut(c).replaced = true;
                }
                if replaced.map_or(true, |c| self.syms.sym(sym).kind != SymKind::Object(c)) {
                    self.syms.pkgs[p.idx()].entries.entry(name).or_default().term = Some(sym);
                }
                old.is_some() && replaced.is_none()
            }
            Owner::Class(c) => {
                self.syms.add_member(c, sym);
                old.is_some()
            }
            Owner::Local => false,
        };
        if duplicate {
            let mut msg = self.already_defined(name, old.unwrap());
            let is_method = |s: SymId| matches!(self.syms.sym(s).kind, SymKind::Def | SymKind::Overloaded(_));
            let is_value = |s: SymId| matches!(self.syms.sym(s).kind, SymKind::Val | SymKind::Var);
            let (o, s) = (old.unwrap(), sym);
            if (is_method(o) && is_value(s)) || (is_value(o) && is_method(s)) {
                msg.push_str("; overloading a value with a method is not supported");
            }
            self.error(span, msg);
        }
        true
    }

    /// Takes a std term out of its package's tables when a definition of the program takes its
    /// name: the entry, the given list, and its definition's check.
    fn retire_std_term(&mut self, p: PkgId, old: SymId) {
        let (name, file, def, kind) = {
            let s = self.syms.sym(old);
            (s.name, s.file, s.def, s.kind)
        };
        let alternatives: Vec<SymId> = self.syms.alternatives(old).map_or(vec![old], |alts| alts.to_vec());
        for s in alternatives {
            if let SymKind::Object(c) = self.syms.sym(s).kind {
                self.syms.class_mut(c).replaced = true;
                self.std_mut().shadowed_classes.insert(c, ());
            }
            if let Some(d) = self.syms.sym(s).def {
                self.def_syms.remove(self.syms.sym(s).file.0 as usize, &d);
            }
            self.syms.pkgs[p.idx()].givens.retain(|&g| g != s);
        }
        let _ = (file, def, kind);
        if let Some(e) = self.syms.pkgs[p.idx()].entries.get_mut(&name) {
            e.term = None;
        }
    }

    /// Whether the def `sym` is a further alternative of what its name stands for so far. The
    /// top-level definitions of one file can be overloaded, those of two files cannot.
    fn overloads(&self, old: SymId, sym: SymId) -> bool {
        let (o, s) = (self.syms.sym(old), self.syms.sym(sym));
        let is_method = |k: SymKind| matches!(k, SymKind::Def | SymKind::Overloaded(_));
        let is_value = |k: SymKind| matches!(k, SymKind::Val | SymKind::Var);
        if s.kind == SymKind::Def && is_method(o.kind) {
            return o.file == s.file || !matches!(s.owner, Owner::Package(_));
        }
        // A val next to a method of its name in a class: alternatives of one name, as scalac
        // keeps them.
        matches!(s.owner, Owner::Class(_)) && ((is_method(o.kind) && is_value(s.kind)) || (is_value(o.kind) && s.kind == SymKind::Def))
    }

    fn add_alternative(&mut self, owner: Owner, old: SymId, sym: SymId) {
        let name = self.syms.sym(sym).name;
        if let SymKind::Overloaded(i) = self.syms.sym(old).kind {
            self.syms.overloads[i as usize].1.push(sym);
            self.syms.sym_mut(sym).alternative = true;
        } else {
            let set = self.syms.new_overloaded(old, owner, vec![old, sym]);
            match owner {
                Owner::Package(p) => self.syms.pkgs[p.idx()].entries.get_mut(&name).unwrap().term = Some(set),
                Owner::Class(c) => {
                    self.syms.class_mut(c).members.insert(name, set);
                }
                Owner::Local => {}
            }
        }
        if let Owner::Class(c) = owner {
            self.syms.class_mut(c).member_order.push(sym);
        }
    }

    /// scalac's E161.
    pub(super) fn already_defined(&self, name: crate::intern::Name, old: SymId) -> String {
        let info = self.syms.sym(old);
        let kind = match info.kind {
            SymKind::Def | SymKind::Overloaded(_) => "method",
            SymKind::Var => "variable",
            SymKind::Object(_) => "object",
            SymKind::Given => "given instance",
            _ => "value",
        };
        let n = self.name_str(name);
        let elsewhere = match info.owner {
            Owner::Package(_) if info.file != self.env.file => {
                format!(" in {}", self.source(info.file).path)
            }
            _ => String::new(),
        };
        format!("{n} is already defined as {kind} {n}{elsewhere}")
    }

    /// Which of two same-named package members is a `@javaDefined` definition of the standard
    /// library: a stand-in for a member of `java.lang`, which a definition of that name in
    /// package `scala` shadows under scalac, so the other one takes its place.
    fn java_stand_in_of(&self, a: ClassId, b: ClassId) -> Option<ClassId> {
        if self.java_stand_in(a) {
            Some(a)
        } else if self.java_stand_in(b) {
            Some(b)
        } else {
            None
        }
    }

    fn java_stand_in(&self, c: ClassId) -> bool {
        let info = self.syms.class(c);
        self.source(info.file).is_std
            && info
                .def
                .map_or(false, |d| self.ast(info.file).def(d).annots.iter().any(|a| a.name == names::JAVA_DEFINED))
    }

    /// Registers a class with its owner; false when a definition of the program shadows this
    /// one of the std, which then stays out of the tables.
    fn register_class(&mut self, owner: Owner, cid: ClassId) -> bool {
        let (name, span, file) = {
            let c = self.syms.class(cid);
            (c.name, c.span, c.file)
        };
        let duplicate = match owner {
            Owner::Package(p) => {
                // The compiler's own types are no classes, so nothing else reports them taken.
                if p == self.b.scala_pkg && matches!(name, names::ANY | names::NOTHING) {
                    let msg = format!("cannot redefine the standard type {}", self.name_str(name));
                    self.error(span, msg);
                    return true;
                }
                let old = self.syms.pkg(p).entries.get(&name).and_then(|e| e.class);
                let taken = old.filter(|&o| self.syms.class(o).kind != ClassKind::Builtin).map(|o| self.syms.class(o).file);
                let taken = taken.or_else(|| self.syms.pkg(p).entries.get(&name).and_then(|e| e.alias).map(|a| self.syms.aliases[a.idx()].file));
                match taken.and_then(|old_file| self.shadowing(old_file, file)) {
                    Some(true) => {
                        self.note_shadowed(owner, name, file);
                        self.syms.class_mut(cid).replaced = true;
                        self.std_mut().shadowed_classes.insert(cid, ());
                        return false;
                    }
                    Some(false) => {
                        self.note_shadowed(owner, name, taken.unwrap());
                        self.retire_std_type(p, name);
                    }
                    None => {}
                }
                self.retire_std_companion_term(p, name, file);
                let (old, has_alias) = {
                    let e = self.syms.pkg(p).entries.get(&name);
                    (e.and_then(|e| e.class), e.map_or(false, |e| e.alias.is_some()))
                };
                // A Java class that a signature named before the std defined it (the platform
                // layer's `java.io.Serializable`) gives way to the std's definition.
                let stands_in = self.source(self.syms.class(cid).file).is_std;
                let replaced = old.and_then(|o| self.java_stand_in_of(o, cid)).or_else(|| old.filter(|&o| stands_in && self.is_java_placeholder(o)));
                if let Some(c) = replaced {
                    self.syms.class_mut(c).replaced = true;
                }
                if replaced != Some(cid) {
                    self.syms.pkgs[p.idx()].entries.entry(name).or_default().class = Some(cid);
                }
                (old.is_some() && replaced.is_none()) || has_alias
            }
            Owner::Class(c) => {
                let mut info = self.syms.class_mut(c);
                info.nested.insert(name, cid).is_some() || info.type_aliases.contains_key(&name)
            }
            Owner::Local => false,
        };
        if duplicate {
            self.report_duplicate_type(name, span);
        }
        true
    }

    /// A std term named like a program class entering package `p` is retired with it: the
    /// program's class and object of one name are one pair.
    fn retire_std_companion_term(&mut self, p: PkgId, name: crate::intern::Name, file: FileId) {
        let Some(o) = self.syms.pkg(p).entries.get(&name).and_then(|e| e.term) else { return };
        let old_file = self.syms.sym(o).file;
        if self.shadowing(old_file, file) == Some(false) {
            self.note_shadowed(Owner::Package(p), name, old_file);
            self.retire_std_term(p, o);
        }
    }

    /// Takes a std class or alias out of its package's entries when a definition of the
    /// program takes its name.
    fn retire_std_type(&mut self, p: PkgId, name: crate::intern::Name) {
        let Some(e) = self.syms.pkgs[p.idx()].entries.get_mut(&name) else { return };
        let (class, alias) = (e.class.take(), e.alias.take());
        if let Some(c) = class {
            self.syms.class_mut(c).replaced = true;
            self.std_mut().shadowed_classes.insert(c, ());
        }
        if let Some(a) = alias {
            let (file, def) = (self.syms.aliases[a.idx()].file, self.syms.aliases[a.idx()].def);
            if let Some(d) = def {
                self.def_aliases.remove(file.0 as usize, &d);
            }
        }
    }

    fn report_duplicate_type(&mut self, name: crate::intern::Name, span: crate::source::Span) {
        let msg = format!("type {} is already defined", self.name_str(name));
        self.error(span, msg);
    }

    pub(super) fn existing_object(&self, owner: Owner, name: crate::intern::Name) -> Option<ClassId> {
        let term = match owner {
            Owner::Package(p) => self.syms.pkg(p).entries.get(&name).and_then(|e| e.term),
            Owner::Class(c) => self.syms.class(c).members.get(&name).copied(),
            Owner::Local => None,
        }?;
        match self.syms.sym(term).kind {
            SymKind::Object(c) => Some(c),
            SymKind::Val => self.inner_object_of_sym(term),
            _ => None,
        }
    }

    pub(super) fn existing_class(&self, owner: Owner, name: crate::intern::Name) -> Option<ClassId> {
        match owner {
            Owner::Package(p) => self.syms.pkg(p).entries.get(&name).and_then(|e| e.class),
            Owner::Class(c) => self.syms.class(c).nested.get(&name).copied(),
            Owner::Local => None,
        }
    }

    pub fn enter_def(&mut self, file: FileId, owner: Owner, id: DefId) {
        let def = self.ast(file).def(id);
        if !def.annots.is_empty() && !matches!(def.kind, DefKind::Val { .. } | DefKind::Fun(_) | DefKind::Class(_)) {
            self.enter_interop_annots(file, owner, def, None);
        }
        match &def.kind {
            DefKind::Val { pat, .. } => {
                if pat.is_some() {
                    self.error(def.span, "pattern definitions are only supported inside blocks");
                    return;
                }
                let kind = if def.mods & mods::MUTABLE != 0 { SymKind::Var } else { SymKind::Val };
                let sym = self.syms.new_sym(def.name, kind, def.mods, owner, file, Some(id), def.span);
                self.note_scoped_private(file, sym);
                self.def_syms.insert(file.0 as usize, id, sym);
                if !self.register_term(owner, sym) {
                    self.def_syms.remove(file.0 as usize, &id);
                    return;
                }
                self.register_implicit(owner, sym);
                self.enter_interop_annots(file, owner, def, Some(sym));
                let abstract_var = kind == SymKind::Var && matches!(def.kind, DefKind::Val { rhs: None, .. }) && def.mods & mods::INCOMPLETE == 0;
                if abstract_var && matches!(owner, Owner::Class(_)) {
                    self.enter_abstract_setter(owner, sym);
                }
            }
            DefKind::Fun(_) if def.name == names::INIT => self.enter_secondary_ctor(file, owner, id),
            DefKind::Fun(f) => {
                // dotty's `Desugar.extMethod`: a right-associative extension method's first
                // clause is the right operand's, a single parameter (`rightAssocParams`); an empty
                // one is no operator's, and stays as it is.
                if f.is_extension && self.interner.get(def.name).ends_with(':') {
                    let own = f.clauses.get(f.ext_clauses as usize);
                    if own.is_some_and(|c| !c.is_using && c.params.len() > 1) {
                        self.error(def.span, "right-associative extension method must start with a single parameter, consider a tupled parameter instead");
                    }
                }
                let sym =
                    self.syms.new_sym(def.name, SymKind::Def, def.mods, owner, file, Some(id), def.span);
                self.note_scoped_private(file, sym);
                let jvm = self.jvm;
                let def_name = if jvm { self.interner.get(def.name).to_string() } else { String::new() };
                {
                    let ast = self.ast(file);
                    let mut info = self.syms.sym_mut(sym);
                    info.is_extension = f.is_extension;
                    info.ext_tparams = f.ext_tparams;
                    info.ext_clauses = f.ext_clauses;
                    for a in &def.annots {
                        if a.name == names::JS && !jvm {
                            info.intrinsic = ast.annot_args(a).first().map(|&s| Arc::from(ast.str(s)));
                        } else if a.name == names::JS {
                            // On the JVM a body takes the place of the JS template; without
                            // one the def is an intrinsic that the backend reports when reached.
                            if info.intrinsic.is_none() && f.body.is_none() {
                                info.intrinsic = Some(Arc::from(format!("!{}", def_name)));
                            }
                        } else if a.name == names::JVM && jvm && !def.annots.iter().any(|a| a.name == names::JVM_LINK) {
                            info.intrinsic = ast.annot_args(a).first().map(|&s| Arc::from(ast.str(s)));
                        } else if a.name == names::JVM_LINK && jvm {
                            // The template over scala-library's bytecode, where `Array` is a JVM
                            // array; the `@jvm` beside it was the retired lean JVM mode's.
                            info.intrinsic = ast.annot_args(a).first().map(|&s| Arc::from(ast.str(s)));
                        } else if a.name == names::MAIN {
                            info.is_main = true;
                        } else if a.name == names::JAVA_DEFINED {
                            info.java_defined = true;
                        } else if a.name == names::JVM_EVIDENCE {
                            info.jvm_evidence = true;
                        }
                    }
                }
                self.def_syms.insert(file.0 as usize, id, sym);
                self.check_intrinsic_receiver(owner, sym);
                if f.is_extension {
                    match owner {
                        Owner::Package(p) => self.syms.pkgs[p.idx()]
                            .entries
                            .entry(def.name)
                            .or_default()
                            .extensions
                            .push(sym),
                        Owner::Class(c) => self.syms.class_mut(c).extensions.push(sym),
                        Owner::Local => {}
                    }
                } else {
                    if !self.register_term(owner, sym) {
                        self.def_syms.remove(file.0 as usize, &id);
                        return;
                    }
                    self.register_implicit(owner, sym);
                }
                self.enter_interop_annots(file, owner, def, Some(sym));
            }
            DefKind::Class(_) => self.enter_class(file, owner, id),
            DefKind::TypeAlias { .. } => {
                if def.mods & mods::OPAQUE != 0 {
                    let cid = self.syms.new_class(
                        def.name,
                        ClassKind::Opaque,
                        def.mods,
                        owner,
                        file,
                        Some(id),
                        def.span,
                    );
                    self.def_classes.insert(file.0 as usize, id, cid);
                    self.file_opaques[file.0 as usize].push(cid);
                    self.mark_opaque_in_class(cid);
                    if !self.register_class(owner, cid) {
                        self.def_classes.remove(file.0 as usize, &id);
                        self.file_opaques[file.0 as usize].pop();
                    }
                } else {
                    let aid = AliasId(self.syms.aliases.len() as u32);
                    let abstract_member = matches!(def.kind, DefKind::TypeAlias { rhs: None, .. });
                    self.syms.aliases.push(AliasInfo {
                        name: def.name,
                        owner,
                        file,
                        def: Some(id),
                        tparams: Vec::new(),
                        rhs: ERROR,
                        bounds: abstract_member.then_some((NOTHING, ANY)),
                    });
                    self.def_aliases.insert(file.0 as usize, id, aid);
                    let duplicate = match owner {
                        Owner::Package(p) => {
                            let taken = {
                                let e = self.syms.pkg(p).entries.get(&def.name);
                                e.and_then(|e| e.class).filter(|&c| self.syms.class(c).kind != ClassKind::Builtin).map(|c| self.syms.class(c).file)
                                    .or_else(|| e.and_then(|e| e.alias).map(|a| self.syms.aliases[a.idx()].file))
                            };
                            match taken.and_then(|old_file| self.shadowing(old_file, file)) {
                                Some(true) => {
                                    self.note_shadowed(owner, def.name, file);
                                    self.def_aliases.remove(file.0 as usize, &id);
                                    return;
                                }
                                Some(false) => {
                                    self.note_shadowed(owner, def.name, taken.unwrap());
                                    self.retire_std_type(p, def.name);
                                }
                                None => {}
                            }
                            let e = self.syms.pkgs[p.idx()].entries.entry(def.name).or_default();
                            let dup = e.alias.is_some() || e.class.is_some();
                            e.alias = Some(aid);
                            dup
                        }
                        Owner::Class(c) => {
                            let mut info = self.syms.class_mut(c);
                            info.type_aliases.insert(def.name, aid).is_some() || info.nested.contains_key(&def.name)
                        }
                        Owner::Local => {
                            self.error(def.span, "local type aliases are not supported");
                            false
                        }
                    };
                    if duplicate {
                        self.report_duplicate_type(def.name, def.span);
                    }
                }
            }
            DefKind::Given(g) => {
                let name = self.given_name(owner, def, g);
                // A given without parameters is a lazy val (or an object) in scalac.
                let parameterless = g.tparams.is_empty() && g.clauses.is_empty();
                let m = if parameterless { def.mods | mods::LAZY } else { def.mods };
                let sym = self.syms.new_sym(name, SymKind::Given, m, owner, file, Some(id), def.span);
                self.note_scoped_private(file, sym);
                self.def_syms.insert(file.0 as usize, id, sym);
                if !self.register_term(owner, sym) {
                    self.def_syms.remove(file.0 as usize, &id);
                    return;
                }
                match owner {
                    Owner::Package(p) => self.syms.pkgs[p.idx()].givens.push(sym),
                    Owner::Class(c) => self.syms.class_mut(c).givens.push(sym),
                    Owner::Local => {}
                }
                if g.alias.is_none() {
                    let cid = self.syms.new_class(
                        name,
                        ClassKind::GivenImpl,
                        mods::FINAL,
                        owner,
                        file,
                        Some(id),
                        def.span,
                    );
                    self.def_classes.insert(file.0 as usize, id, cid);
                    self.syms.sym_mut(sym).impl_class = Some(cid);
                    self.enter_ctor_params(file, cid, &g.clauses);
                    self.enter_body(file, cid, &g.body);
                    self.record_self_alias(cid);
                }
            }
        }
    }

    /// `def this(...)`: an alternative of the constructor, which is no member.
    fn enter_secondary_ctor(&mut self, file: FileId, owner: Owner, id: DefId) {
        let def = self.ast(file).def(id);
        let c = match owner {
            Owner::Class(c) if self.syms.class(c).kind == ClassKind::Class => c,
            Owner::Class(c) if self.syms.class(c).kind == ClassKind::Trait => {
                self.error(def.span, "Traits cannot have secondary constructors");
                return;
            }
            _ => {
                self.error(def.span, "a secondary constructor can only be defined in a class");
                return;
            }
        };
        let sym = self.syms.new_sym(names::INIT, SymKind::Def, def.mods, owner, file, Some(id), def.span);
        self.note_scoped_private(file, sym);
        self.def_syms.insert(file.0 as usize, id, sym);
        self.syms.class_mut(c).ctors.push(sym);
        self.ensure_primary_ctor(c);
    }

    /// The symbol of the primary constructor, next to the secondaries as an alternative; its
    /// signature is sealed when the class completes.
    pub(super) fn ensure_primary_ctor(&mut self, c: ClassId) {
        if self.syms.class(c).primary_ctor.is_some() {
            return;
        }
        let (file, span, m) = {
            let i = self.syms.class(c);
            (i.file, i.span, if i.mods & mods::PRIVATE_CTOR != 0 { mods::PRIVATE } else { 0 })
        };
        let sym = self.syms.new_sym(names::INIT, SymKind::Def, m, Owner::Class(c), file, None, span);
        self.syms.class_mut(c).primary_ctor = Some(sym);
    }

    /// A Scala 2 implicit definition joins the givens of its scope, and so does an abstract
    /// given (`given x: T`, a def flagged `Given`).
    fn register_implicit(&mut self, owner: Owner, sym: SymId) {
        if self.syms.sym(sym).mods & (mods::IMPLICIT | mods::GIVEN) == 0 {
            return;
        }
        match owner {
            Owner::Package(p) => self.syms.pkgs[p.idx()].givens.push(sym),
            Owner::Class(c) => self.syms.class_mut(c).givens.push(sym),
            Owner::Local => {}
        }
    }

    /// A template that spells `$0` for the first parameter would silently get the object.
    fn check_intrinsic_receiver(&mut self, owner: Owner, sym: SymId) {
        let Owner::Class(c) = owner else { return };
        let info = self.syms.sym(sym);
        let takes_object = self.syms.class(c).kind == ClassKind::Object && !info.is_extension;
        if takes_object && info.intrinsic.as_deref().map_or(false, |t| t.contains("$0")) {
            let span = info.span;
            self.error(span, "in a member of an object $0 is the object itself; the parameters start at $1");
        }
    }

    /// scalac gives `given [T: TextKey] => Eq[T] = ..` and `given [T: LongKey] => Eq[T] = ..`
    /// the same name, which it can do because they are overloaded methods. Here the first one
    /// keeps that name and the others get a suffix; nothing refers to them by name.
    fn given_name(&mut self, owner: Owner, def: &ast::Def, g: &ast::GivenDef) -> crate::intern::Name {
        let is_method = g.alias.is_some() && !(g.tparams.is_empty() && g.clauses.is_empty());
        let overloadable = def.mods & mods::ANONYMOUS != 0 && is_method;
        if !overloadable || !self.names_given(owner, def.name) {
            return def.name;
        }
        let base = self.name_str(def.name);
        let mut n = 1;
        loop {
            let name = self.interner.intern(&format!("{}${}", base, n));
            if !self.names_given(owner, name) {
                return name;
            }
            n += 1;
        }
    }

    fn names_given(&self, owner: Owner, name: crate::intern::Name) -> bool {
        let term = match owner {
            Owner::Package(p) => self.syms.pkg(p).entries.get(&name).and_then(|e| e.term),
            Owner::Class(c) => self.syms.class(c).members.get(&name).copied(),
            Owner::Local => None,
        };
        term.map_or(false, |s| self.syms.sym(s).kind == SymKind::Given)
    }

    /// The setter `x_=(x$1: T): Unit` of an abstract `var x` of a class, abstract and without a
    /// definition (`mods::SETTER`), whose signature the var's gives (`complete_sig_inner`). The
    /// var is read through its getter, as a def implementing it is.
    fn enter_abstract_setter(&mut self, owner: Owner, var: SymId) {
        let (name, span, file, var_mods, scoped_private) = {
            let v = self.syms.sym(var);
            (v.name, v.span, v.file, v.mods, v.scoped_private)
        };
        let setter_name = self.interner.intern(&format!("{}_=", self.interner.get(name)));
        let m = mods::ABSTRACT | mods::SETTER | var_mods & (mods::PRIVATE | mods::PROTECTED);
        let setter = self.syms.new_sym(setter_name, SymKind::Def, m, owner, file, None, span);
        self.syms.sym_mut(setter).scoped_private = scoped_private;
        self.syms.sym_mut(var).needs_accessor = true;
        self.register_term(owner, setter);
    }

    fn note_scoped_private(&mut self, file: FileId, sym: SymId) {
        let info = self.syms.sym(sym);
        let scopes = &self.ast(file).access_scopes;
        if info.mods & mods::PRIVATE != 0 && !scopes.is_empty() && scopes.iter().any(|&(at, _)| at == info.span.start) {
            self.syms.sym_mut(sym).scoped_private = true;
        }
    }

    fn enter_ctor_params(&mut self, file: FileId, cid: ClassId, clauses: &[ast::ParamClause]) {
        // A native JS class has no fields of its own: a plain parameter only shapes the
        // constructor call and may share its name with a member.
        let native = self.syms.class(cid).js == JsKind::Native;
        for clause in clauses {
            let mut syms = Vec::with_capacity(clause.params.len());
            for p in &clause.params {
                let kind = if p.mods & mods::MUTABLE != 0 { SymKind::Var } else { SymKind::Val };
                let mut m = p.mods;
                if m & mods::FIELD == 0 {
                    m |= mods::PRIVATE;
                }
                let sym = self.syms.new_sym(p.name, kind, m, Owner::Class(cid), file, None, p.span);
                self.note_scoped_private(file, sym);
                if !native || m & mods::FIELD != 0 {
                    self.register_term(Owner::Class(cid), sym);
                }
                self.enter_param_annots(file, p, sym);
                if clause.is_using {
                    self.syms.class_mut(cid).givens.push(sym);
                }
                syms.push(sym);
            }
            self.syms.class_mut(cid).ctor_syms.push(syms);
        }
    }

    pub(super) fn enter_body(&mut self, file: FileId, cid: ClassId, body: &[Stmt]) {
        let in_trait = self.syms.class(cid).kind == ClassKind::Trait;
        for stmt in body {
            match stmt {
                Stmt::Def(d) => {
                    // The initialiser of a val of a trait runs with the statements of its body.
                    let def = self.ast(file).def(*d);
                    if in_trait && def.mods & mods::LAZY == 0 && matches!(def.kind, DefKind::Val { rhs: Some(_), .. }) {
                        self.syms.class_mut(cid).has_statements = true;
                    }
                    self.enter_def(file, Owner::Class(cid), *d)
                }
                Stmt::Import(_) => self.syms.class_mut(cid).has_imports = true,
                Stmt::Expr(_) => self.syms.class_mut(cid).has_statements = true,
            }
        }
    }

    fn enter_class(&mut self, file: FileId, owner: Owner, id: DefId) {
        let def = self.ast(file).def(id);
        let DefKind::Class(cls) = &def.kind else { return };
        // A class overrides an abstract type member at most, which is not in the language.
        if def.mods & mods::OVERRIDE != 0 && cls.kind != ast::ClassKind::Object {
            let kind = if cls.kind == ast::ClassKind::Trait { "trait" } else { "class" };
            let msg = format!("{} {} overrides nothing", kind, self.name_str(def.name));
            self.error(def.span, msg);
        }
        match cls.kind {
            // An object nested in a class or trait is one instance per enclosing instance; the
            // companion of an enum nested there stays a single object, where the enum's cases
            // are made.
            ast::ClassKind::Object if matches!(owner, Owner::Class(o) if self.syms.class(o).kind != ClassKind::Object && !self.enum_named_in_body(o, def.name)) => {
                self.enter_inner_object(file, owner, id);
            }
            ast::ClassKind::Object => {
                let cid = match self.existing_object(owner, def.name) {
                    // An enum's synthesized companion gets merged with an explicit object of
                    // its file; an object of another file with that name is another definition.
                    Some(existing) if self.syms.class(existing).def.is_none() && self.syms.class(existing).file == file => {
                        self.syms.class_mut(existing).def = Some(id);
                        existing
                    }
                    _ => self.new_object(file, owner, def.name, def.mods, Some(id), def.span),
                };
                self.def_classes.insert(file.0 as usize, id, cid);
                if self.shadowed_object == Some(cid) {
                    return;
                }
                if let (Owner::Package(p), names::PACKAGE) = (owner, def.name) {
                    self.syms.pkgs[p.idx()].package_object = Some(cid);
                }
                self.syms.class_mut(cid).has_exports |= !cls.exports.is_empty();
                self.enter_class_annots(file, def, cid);
                if let Some(module) = self.syms.class(cid).module_sym {
                    self.register_implicit(owner, module);
                }
                if let Some(class) = self.existing_class(owner, def.name) {
                    self.syms.class_mut(class).companion = Some(cid);
                    self.syms.class_mut(cid).companion = Some(class);
                }
                self.enter_body(file, cid, &cls.body);
            }
            ast::ClassKind::Class | ast::ClassKind::Trait | ast::ClassKind::Enum => {
                let kind = match cls.kind {
                    ast::ClassKind::Class => ClassKind::Class,
                    ast::ClassKind::Trait => ClassKind::Trait,
                    _ => ClassKind::Enum,
                };
                let cid =
                    self.syms.new_class(def.name, kind, def.mods, owner, file, Some(id), def.span);
                self.def_classes.insert(file.0 as usize, id, cid);
                self.syms.class_mut(cid).has_exports = !cls.exports.is_empty();
                self.record_self_alias(cid);
                if !self.register_class(owner, cid) {
                    return;
                }
                self.enter_class_annots(file, def, cid);
                self.enter_ctor_params(file, cid, &cls.clauses);
                if let Some(obj) = self.existing_object(owner, def.name) {
                    self.syms.class_mut(cid).companion = Some(obj);
                    self.syms.class_mut(obj).companion = Some(cid);
                }
                if kind == ClassKind::Enum {
                    self.enter_enum_body(file, owner, cid, id);
                } else {
                    self.enter_body(file, cid, &cls.body);
                }
            }
            ast::ClassKind::EnumCase => {}
        }
    }

    /// An object nested in a class or trait: a final inner class taking the enclosing instance,
    /// as any class nested there does, and a lazy val of the outer holding the one instance per
    /// enclosing instance, as scalac's module val (`ClassInfo::inner_object`; the loader
    /// enters a jar's the same way). The class is not among the outer's nested classes, so
    /// that a companion class of the name stands there.
    fn enter_inner_object(&mut self, file: FileId, owner: Owner, id: DefId) {
        let def = self.ast(file).def(id);
        let DefKind::Class(cls) = &def.kind else { return };
        let cid = self.syms.new_class(def.name, ClassKind::Class, def.mods | mods::FINAL, owner, file, Some(id), def.span);
        self.def_classes.insert(file.0 as usize, id, cid);
        self.syms.class_mut(cid).has_exports = !cls.exports.is_empty();
        self.record_self_alias(cid);
        self.enter_class_annots(file, def, cid);
        let ty = self.types.class(cid, &[]);
        let m = def.mods & (mods::PRIVATE | mods::PROTECTED | mods::IMPLICIT | mods::QUALIFIED) | mods::FINAL | mods::LAZY;
        let sym = self.syms.new_sym(def.name, SymKind::Val, m, owner, file, None, def.span);
        {
            let mut s = self.syms.sym_mut(sym);
            s.sig = Some(Arc::new(MethodSig::value(ty)));
            s.state().set(Completion::Done);
        }
        self.syms.class_mut(cid).inner_object = Some(sym);
        self.note_scoped_private(file, sym);
        if !self.register_term(owner, sym) {
            return;
        }
        self.register_implicit(owner, sym);
        if let Some(class) = self.existing_class(owner, def.name) {
            self.syms.class_mut(class).companion = Some(cid);
            self.syms.class_mut(cid).companion = Some(class);
        }
        self.enter_body(file, cid, &cls.body);
    }

    /// Whether the body of the class `o` defines an enum called `name`, whose companion an
    /// object of that name is.
    fn enum_named_in_body(&self, o: ClassId, name: crate::intern::Name) -> bool {
        let info = self.syms.class(o);
        let Some(d) = info.def else { return false };
        let ast = self.ast(info.file);
        let DefKind::Class(cls) = &ast.def(d).kind else { return false };
        cls.body.iter().any(|s| match s {
            Stmt::Def(m) => {
                let def = ast.def(*m);
                def.name == name && matches!(&def.kind, DefKind::Class(c) if c.kind == ast::ClassKind::Enum)
            }
            _ => false,
        })
    }

    fn new_object(
        &mut self,
        file: FileId,
        owner: Owner,
        name: crate::intern::Name,
        m: crate::ast::Mods,
        def: Option<DefId>,
        span: crate::source::Span,
    ) -> ClassId {
        let cid = self.syms.new_class(name, ClassKind::Object, m | mods::FINAL, owner, file, def, span);
        self.record_self_alias(cid);
        let sym = self.syms.new_sym(name, SymKind::Object(cid), m, owner, file, None, span);
        let ty = self.types.class(cid, &[]);
        {
            let mut s = self.syms.sym_mut(sym);
            s.sig = Some(Arc::new(MethodSig::value(ty)));
            s.state().set(Completion::Done);
        }
        self.syms.class_mut(cid).module_sym = Some(sym);
        if !self.register_term(owner, sym) {
            self.shadowed_object = Some(cid);
        }
        cid
    }

    fn enter_enum_body(&mut self, file: FileId, owner: Owner, enum_cid: ClassId, id: DefId) {
        let def = self.ast(file).def(id);
        let DefKind::Class(cls) = &def.kind else { return };
        let companion = match self.syms.class(enum_cid).companion {
            Some(c) => c,
            None => {
                let c = self.new_object(file, owner, def.name, 0, None, def.span);
                self.syms.class_mut(enum_cid).companion = Some(c);
                self.syms.class_mut(c).companion = Some(enum_cid);
                c
            }
        };
        let mut ordinal = 0u32;
        for stmt in &cls.body {
            if let Stmt::Import(_) = stmt {
                self.syms.class_mut(enum_cid).has_imports = true;
            }
            let Stmt::Def(d) = stmt else { continue };
            let member = self.ast(file).def(*d);
            let is_case = matches!(&member.kind, DefKind::Class(c) if c.kind == ast::ClassKind::EnumCase);
            if !is_case {
                self.enter_def(file, Owner::Class(enum_cid), *d);
                continue;
            }
            let DefKind::Class(case_cls) = &member.kind else { continue };
            let case_cid = self.syms.new_class(
                member.name,
                ClassKind::EnumCase,
                member.mods,
                Owner::Class(companion),
                file,
                Some(*d),
                member.span,
            );
            self.def_classes.insert(file.0 as usize, *d, case_cid);
            self.syms.class_mut(case_cid).ordinal = ordinal;
            ordinal += 1;
            self.syms.class_mut(enum_cid).children.push(case_cid);
            if case_cls.clauses.is_empty() {
                let sym = self.syms.new_sym(
                    member.name,
                    SymKind::EnumValue(case_cid),
                    0,
                    Owner::Class(companion),
                    file,
                    None,
                    member.span,
                );
                self.syms.class_mut(case_cid).singleton = Some(sym);
                self.register_term(Owner::Class(companion), sym);
            } else {
                self.register_class(Owner::Class(companion), case_cid);
                self.enter_ctor_params(file, case_cid, &case_cls.clauses);
            }
        }
    }
}
