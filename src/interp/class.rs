//! Classes at run time: the construction protocol both backends follow, module and top-level
//! initialisation, field slots keyed by the name the JavaScript output would use, and member
//! dispatch in the order the prototype chain answers it.

use super::registry;
use super::value::*;
use super::*;
use crate::ast::{mods, ListRef};
use std::cell::Cell;
use crate::symbols::ClassKind;
use crate::typer::{TermRef, TypeRef};
use crate::tir::*;
use crate::types::Type;

impl<'a, 't> Interp<'a, 't> {
    // ---- TClass access ----

    fn refresh_tclasses(&mut self) {
        let n = self.prog().classes.entries().count();
        let known = self.tclass_of.iter().filter(|&&i| i != u32::MAX).count();
        if known == n {
            return;
        }
        let ids: Vec<(u32, usize)> = self.prog().classes.entries().map(|(i, tc)| (i, tc.id.idx())).collect();
        for (i, idx) in ids {
            self.tclass_of[idx] = i;
        }
    }

    /// The `TClass` of `c`, typed now when it comes from a jar and the program has not reached
    /// it before.
    pub(super) fn tclass_index(&mut self, c: ClassId) -> Option<usize> {
        if self.macro_ctx.is_some() {
            self.note_macro_class(c);
        }
        if self.tclass_of.get(c.idx()).map_or(true, |&i| i == u32::MAX) {
            self.refresh_tclasses();
        }
        if let Some(&i) = self.tclass_of.get(c.idx()).filter(|&&i| i != u32::MAX) {
            return Some(i as usize);
        }
        // Another worker's, published with its check: read through that worker's chunk, and
        // not entered in `tclass_of`, which `refresh_tclasses` counts against the entries
        // this worker's view lists.
        if let Some(&i) = self.typer.prog.class_bodies.get(&c) {
            crate::typer::bundle::entered_at(&self.typer.prog.classes, i, crate::typer::bundle::Entry::Class);
            return Some(i as usize);
        }
        if !self.typer.is_library_class(c) {
            // A class the check pass has not reached yet, met while a macro runs.
            if self.syms().class(c).def.is_none() || self.syms().class(c).owner == Owner::Local {
                return None;
            }
            // A class the interpreter reaches is every worker's: checked under the loader's
            // lock into the shared region, its members' bodies with it.
            if std::env::var_os("TEQ_MERGE_TRACE").is_some_and(|v| v == "print") && self.typer.forked {
                let file = self.syms().class(c).file;
                eprintln!("merge check: the interpreter checks class {} of {} on worker {}", self.typer.name_str(self.syms().class(c).name), self.typer.source(file).path, self.typer.worker);
            }
            if self.typer.program_file(self.syms().class(c).file) {
                // A program class is checked outside the lock like the walk's,
                // into this worker's chunk, or found in the chunk of
                // the worker that checked it.
                self.typer.check_class_for_run(c);
            } else {
                self.typer.with_loader(|t| t.check_class_for_run(c));
            }
            self.refresh_tclasses();
            self.note_withheld();
            if let Some(&i) = self.tclass_of.get(c.idx()).filter(|&&i| i != u32::MAX) {
                return Some(i as usize);
            }
            return self.typer.prog.class_bodies.get(&c).map(|&i| i as usize);
        }
        self.typer.check_library_class(c);
        self.note_withheld();
        self.refresh_tclasses();
        if let Some(&i) = self.tclass_of.get(c.idx()).filter(|&&i| i != u32::MAX) {
            return Some(i as usize);
        }
        self.typer.prog.class_bodies.get(&c).map(|&i| i as usize)
    }

    pub(super) fn tclass_field<T>(&mut self, c: ClassId, f: impl Fn(&TClass) -> T) -> Option<T> {
        let i = self.tclass_index(c)?;
        Some(f(&self.prog().classes[i]))
    }

    /// How many leading constructor arguments of `c` are captured locals: all of an anonymous
    /// class's, the leading ones of a named local class.
    pub(super) fn capture_count(&mut self, c: ClassId) -> usize {
        let anon = self.syms().class(c).kind == ClassKind::Anon;
        self.tclass_field(c, |tc| if anon { tc.ctor_params.len() } else { tc.captures }).unwrap_or(0)
    }

    // ---- modules ----

    pub(super) fn module(&mut self, c: ClassId) -> R {
        if let Some(o) = self.made.module(c) {
            return Ok(Value::Obj(o.clone()));
        }
        if self.syms().class(c).js_binding.is_some() {
            let name = self.class_path(c);
            return self.unsupported(format!("{} is a JavaScript object", name));
        }
        let java_statics = self.typer.is_java_class(c) && self.tclass_index(c).is_none();
        // The module is made and initialised in an epoch of its own (`super::watch`).
        let outer = super::enter_epoch();
        let _init = super::enter_init(|| self.class_path(c));
        let cache = match super::watching() && self.typer.cacheable_object(c) {
            false => super::Cache::None,
            true if self.typer.known_cache_object(c) => super::Cache::Known,
            true => super::Cache::Declared,
        };
        super::own_epoch(super::EpochOwner::Module(cache, c), || self.class_path(c));
        let obj = Rc::new(Object {
            class: c,
            fields: RefCell::new(Vec::new()),
            env: None,
            name: None,
            ordinal: Cell::new(0),
            hash: Cell::new(0),
            slot: Cell::new(0),
            born: Cell::new(0),
            made: Cell::new(super::made_now()),
        });
        let classes = self.syms().classes.len();
        self.made.set_module(c, obj.clone(), self.records.runs, classes);
        // The statics of a Java class: its members are the natives the interpreter has for them.
        if java_statics {
            super::leave_epoch(outer);
            return Ok(Value::Obj(obj));
        }
        let r = self.enter().and_then(|_| {
            let r = self.construct(&obj, c, Vec::new());
            self.depth -= 1;
            r
        });
        super::stamp(&Value::Obj(obj.clone()), super::epoch());
        super::leave_epoch(outer);
        // A module whose initialiser failed is not done: a later run under a larger budget
        // builds it again rather than reading its absent fields.
        if let Err(f) = r {
            self.made.clear_module(c);
            return Err(f);
        }
        Ok(Value::Obj(obj))
    }

    // ---- top-level values ----

    /// The shared list and this worker's own read by a cursor each: what the prefix and the
    /// cells registered is every worker's, what the bodies register the worker's.
    fn refresh_top_vals(&mut self) {
        let (shared, own) = (self.prog().top_vals.shared_len(), self.prog().top_vals.own().len());
        if (shared, own) == self.top_vals_seen {
            return;
        }
        for i in self.top_vals_seen.0..shared {
            let (sym, init) = self.prog().top_vals.shared_get(i);
            self.top_val_of.insert(sym, init);
        }
        for i in self.top_vals_seen.1..own {
            let (sym, init) = self.prog().top_vals.own()[i];
            self.top_val_of.insert(sym, init);
        }
        self.top_vals_seen = (shared, own);
        self.file_vals.clear();
    }

    fn is_literal(&self, e: TExprId) -> bool {
        matches!(
            self.prog().expr(e),
            TExpr::Int(_) | TExpr::Double(_) | TExpr::Bool(_) | TExpr::Str(_) | TExpr::Char(_) | TExpr::Long(_)
        )
    }

    /// A top-level val that the initialiser of its file runs: lazy vals, givens, constants and
    /// the vals the compiler makes are read on their own.
    fn is_eager_top_val(&mut self, s: SymId) -> bool {
        let info = self.syms().sym(s);
        if !matches!(info.kind, SymKind::Val | SymKind::Var) || info.mods & mods::LAZY != 0 || info.def.is_none() {
            return false;
        }
        let kind = info.kind;
        match self.top_val(s) {
            Some(init) => !(self.is_literal(init) && kind == SymKind::Val),
            None => false,
        }
    }

    /// The initialiser of a top-level val: registered, or typed where the walk that registers
    /// it has not come yet, or by another worker, whose walk registers it.
    fn top_val(&self, s: SymId) -> Option<TExprId> {
        self.top_val_of.get(&s).copied().or_else(|| {
            let e = *self.typer.val_init.get(&s)?;
            crate::typer::bundle::entered_at(&self.typer.prog.exprs, e.0, crate::typer::bundle::Entry::Val);
            Some(e)
        })
    }

    /// The eager vals of a file in source order: every val the file defines at the top level,
    /// the ones no walk got to yet typed now, so that the file's initialiser runs the same
    /// vals whatever was typed when.
    fn file_vals(&mut self, file: FileId) -> Vec<(SymId, TExprId)> {
        if let Some(v) = self.file_vals.get(&file) {
            return v.clone();
        }
        let defined = self.typer.top_level_vals(file);
        for &s in &defined {
            if self.top_val(s).is_none() && self.typer.program_file(file) {
                self.typer.ensure_body(s);
            }
        }
        let mut vals: Vec<(SymId, TExprId)> = self.prog().top_vals.all().filter(|&(s, _)| self.syms().sym(s).file == file).collect();
        for s in defined {
            if !vals.iter().any(|&(v, _)| v == s) {
                if let Some(init) = self.top_val(s) {
                    vals.push((s, init));
                }
            }
        }
        vals.retain(|&(s, _)| self.is_eager_top_val(s));
        vals.sort_by_key(|&(s, _)| {
            let info = self.syms().sym(s);
            (info.span.start, info.def.map_or(u32::MAX, |d| d.0), s.0)
        });
        self.file_vals.insert(file, vals.clone());
        vals
    }

    pub(super) fn run_file_init(&mut self, file: FileId) -> R<()> {
        self.refresh_top_vals();
        if self.macro_ctx.is_some() {
            self.typer.macro_files.insert(file, ());
        }
        let f = file.0 as usize;
        if self.made.file_done.len() <= f {
            self.made.file_done.resize(f + 1, false);
        }
        if self.made.file_done[f] {
            return Ok(());
        }
        self.made.file_done[f] = true;
        let vals = self.file_vals(file);
        self.halt_on_withheld()?;
        if vals.is_empty() {
            return Ok(());
        }
        // The file's initialiser runs in an epoch of its own (`super::watch`), and initialises
        // the slots of its vals, which it makes.
        let outer = super::enter_epoch();
        let init = super::enter_init(|| format!("the file {}", self.typer.source(file).path));
        super::own_epoch(super::EpochOwner::Other, || format!("the file {}", self.typer.source(file).path));
        let syms: Vec<SymId> = if super::watching() { vals.iter().map(|&(s, _)| s).collect() } else { Vec::new() };
        let r = self.run_file_vals(f, vals);
        drop(init);
        let e = super::epoch();
        for s in syms {
            self.made.stamp_static(s, e);
        }
        super::leave_epoch(outer);
        r
    }

    fn run_file_vals(&mut self, f: usize, vals: Vec<(SymId, TExprId)>) -> R<()> {
        let frame = Frame::new(None);
        let cx = Ctx::plain();
        for (sym, init) in vals {
            // A val kept from an earlier build with the object of cacheable state that reaches its
            // value (`forget_made`) is not made again: what reached it stays the one value.
            if self.made.statics.contains_key(&sym) {
                continue;
            }
            match self.eval(init, &frame, &cx) {
                Ok(v) => {
                    self.made.insert_static(sym, v, self.records.runs);
                }
                Err(e) => {
                    self.made.file_done[f] = false;
                    return Err(e);
                }
            }
        }
        Ok(())
    }

    /// The file of a top-level definition: a product's is converted and checked with its file's
    /// eager group first, which places it in that file.
    fn top_def_file(&mut self, s: SymId, file: FileId) -> FileId {
        if self.top_val(s).is_some() || self.typer.fun_of_sym.contains_key(&s) || !self.typer.is_product_top_def(s) {
            return file;
        }
        let _ = self.typer.deferred_body(s);
        self.note_withheld();
        self.syms().sym(s).file
    }

    /// Runs the initialiser of the file of `s` where an access of `s` runs it first, ahead of
    /// the arguments of a call and of an assigned value (`layout::initialises_file`). A std
    /// file's initialisers are pure: its triggers are the reads of its eager vals alone.
    #[inline]
    pub(super) fn init_file_of(&mut self, s: SymId, owner: Owner, file: FileId) -> R<()> {
        let f = file.0 as usize;
        if !matches!(owner, Owner::Package(_)) || self.made.file_done.get(f).copied().unwrap_or(false) || self.made.file_quiet.get(f).copied().unwrap_or(false) {
            return Ok(());
        }
        self.init_file_now(s, file)
    }

    #[cold]
    fn init_file_now(&mut self, s: SymId, file: FileId) -> R<()> {
        if self.note_quiet(file) {
            return Ok(());
        }
        let file = self.top_def_file(s, file);
        let folded = self.typer.constant_value(s).is_some();
        if !crate::emit::layout::initialises_file(self.syms(), s, folded) {
            return Ok(());
        }
        self.run_file_init(file)
    }

    /// Runs the initialiser of the file of the given class `c` where making an instance of it
    /// does so first (`layout::given_class_initialises_file`).
    #[inline]
    pub(super) fn init_file_of_class(&mut self, c: ClassId) -> R<()> {
        let info = self.syms().class(c);
        let f = info.file.0 as usize;
        if info.kind != ClassKind::GivenImpl || self.made.file_done.get(f).copied().unwrap_or(false) || self.made.file_quiet.get(f).copied().unwrap_or(false) {
            return Ok(());
        }
        self.init_given_file(c)
    }

    #[cold]
    fn init_given_file(&mut self, c: ClassId) -> R<()> {
        let file = self.syms().class(c).file;
        if self.note_quiet(file) || !crate::emit::layout::given_class_initialises_file(self.syms(), c) {
            return Ok(());
        }
        self.run_file_init(file)
    }

    /// Whether `file` is a std file, whose accesses run nothing of its initialiser's but the
    /// reads of its vals; recorded for the next access's check.
    fn note_quiet(&mut self, file: FileId) -> bool {
        if !self.typer.std_source(file) {
            return false;
        }
        let f = file.0 as usize;
        if self.made.file_quiet.len() <= f {
            self.made.file_quiet.resize(f + 1, false);
        }
        self.made.file_quiet[f] = true;
        true
    }

    pub(super) fn static_value(&mut self, s: SymId) -> R {
        if let Some(v) = self.made.statics.get(&s) {
            return Ok(v.clone());
        }
        let info = self.syms().sym(s);
        let (kind, owner, file) = (info.kind, info.owner, info.file);
        match (kind, owner) {
            (SymKind::EnumValue(case), _) => self.enum_value(case, s),
            (SymKind::Object(c), _) => self.module(c),
            (_, Owner::Class(c)) => {
                let module = self.module(c)?;
                let name = self.syms().dispatch_name(s);
                match self.member(c, name) {
                    m @ Member::Fun(..) => self.invoke_member(module, m, name, Vec::new()),
                    _ => self.get_field(module, s),
                }
            }
            (SymKind::Def, _) => {
                self.init_file_of(s, owner, file)?;
                self.call_static(s, Vec::new())
            }
            (_, Owner::Package(_)) => {
                let file = self.top_def_file(s, file);
                self.refresh_top_vals();
                if self.macro_ctx.is_some() {
                    self.typer.macro_files.insert(file, ());
                }
                // A val of a file no walk has reached yet, or another worker's walk is typing, is
                // typed here or waited for, as a file's initialiser has its vals (`file_vals`).
                if self.top_val(s).is_none() && self.typer.program_file(file) {
                    self.typer.ensure_body(s);
                }
                if self.is_eager_top_val(s) {
                    self.run_file_init(file)?;
                    return Ok(self.made.statics.get(&s).cloned().unwrap_or(Value::Null));
                }
                self.init_file_of(s, owner, file)?;
                if let Some(v) = self.made.statics.get(&s) {
                    return Ok(v.clone());
                }
                let init = match self.top_val(s) {
                    Some(init) => init,
                    None => match self.typer.deferred_body(s) {
                        Some(crate::typer::check::DeferredBody::Val(init)) => init,
                        Some(crate::typer::check::DeferredBody::Fun(_)) => return self.call_static(s, Vec::new()),
                        None => {
                            let name = self.qualified_name(s);
                            return self.unsupported(format!("{} has no value to read", name));
                        }
                    },
                };
                self.halt_on_withheld()?;
                let outer = super::enter_epoch();
                let cx = super::enter_init(|| self.qualified_name(s));
                super::own_epoch(super::EpochOwner::Other, || self.qualified_name(s).to_string());
                let frame = Frame::new(None);
                let v = self.eval(init, &frame, &Ctx::plain());
                drop(cx);
                let e = super::epoch();
                super::leave_epoch(outer);
                let v = v?;
                self.made.insert_static(s, v.clone(), self.records.runs);
                self.made.stamp_static(s, e);
                Ok(v)
            }
            _ => {
                let name = self.qualified_name(s);
                self.unsupported(format!("{} cannot be read statically", name))
            }
        }
    }

    pub(super) fn set_static(&mut self, s: SymId, v: Value) -> R<()> {
        if self.pure && self.macro_ctx.is_none() {
            return self.impure("an assignment to a top-level variable");
        }
        let info = self.syms().sym(s);
        match info.owner {
            Owner::Class(c) => {
                let module = self.module(c)?;
                self.set_field(module, s, v)
            }
            _ => {
                let file = self.top_def_file(s, info.file);
                self.refresh_top_vals();
                if self.is_eager_top_val(s) {
                    self.run_file_init(file)?;
                }
                if !self.made.static_made.is_empty() {
                    let born = self.made.static_born.get(&s).copied().unwrap_or(0);
                    let made = self.made.static_made.get(&s).copied().unwrap_or(0);
                    super::written(born, made, || None, |_| format!("the top-level variable {}", self.qualified_name(s)));
                }
                self.made.insert_static(s, v, self.records.runs);
                Ok(())
            }
        }
    }

    // ---- enum values ----

    /// The enum that a case belongs to.
    pub(super) fn enum_of_case(&self, case: ClassId) -> Option<ClassId> {
        let Owner::Class(companion) = self.syms().class(case).owner else { return None };
        self.syms().class(companion).companion
    }

    fn make_enum_value(&mut self, case: ClassId, sym: SymId) -> R {
        let outer = super::enter_epoch();
        let init = super::enter_init(|| self.qualified_name(sym));
        super::own_epoch(super::EpochOwner::Other, || self.qualified_name(sym).to_string());
        let r = self.make_enum_value_now(case, sym);
        drop(init);
        self.made.stamp_static(sym, super::epoch());
        super::leave_epoch(outer);
        r
    }

    fn make_enum_value_now(&mut self, case: ClassId, sym: SymId) -> R {
        let info = self.syms().class(case);
        let name: Rc<str> = Rc::from(self.name(info.name));
        let ordinal = info.ordinal as i32;
        let obj = Rc::new(Object {
            class: case,
            fields: RefCell::new(Vec::new()),
            env: None,
            name: Some(name),
            ordinal: Cell::new(ordinal),
            hash: Cell::new(0), slot: Cell::new(0), born: Cell::new(0), made: Cell::new(super::made_now()) });
        let v = Value::Obj(obj.clone());
        self.made.insert_static(sym, v.clone(), self.records.runs);
        self.enter()?;
        let r = self.construct(&obj, case, Vec::new());
        self.depth -= 1;
        r?;
        Ok(v)
    }

    /// An enum value is created on its first read, or by its companion when the enum is
    /// stateful; a companion with a body is touched first, as scalac reads the value from it.
    fn enum_value(&mut self, case: ClassId, sym: SymId) -> R {
        let Owner::Class(companion) = self.syms().sym(sym).owner else { return self.make_enum_value(case, sym) };
        let stateful = self.enum_of_case(case).map_or(false, |e| self.syms().class(e).stateful);
        if stateful {
            self.module(companion)?;
            return Ok(self.made.statics.get(&sym).cloned().unwrap_or(Value::Null));
        }
        let v = self.make_enum_value(case, sym)?;
        if self.has_body(companion) {
            self.module(companion)?;
        }
        Ok(v)
    }

    // ---- construction ----

    pub(super) fn construct_new(&mut self, c: ClassId, args: Vec<Value>, fr: &Rc<Frame>) -> R {
        let info = self.syms().class(c);
        let kind = info.kind;
        if kind == ClassKind::Builtin {
            let obj = Rc::new(Object { class: c, fields: RefCell::new(Vec::new()), env: None, name: None, ordinal: Cell::new(0), hash: Cell::new(0), slot: Cell::new(0), born: Cell::new(0), made: Cell::new(super::made_now()) });
            return Ok(Value::Obj(obj));
        }
        self.touch_companion(c)?;
        self.prof_alloc(|a| a.objects += 1);
        let plan = self.ctor_plan(c);
        self.halt_on_withheld()?;
        let args = match plan {
            Some(slots) => match self.plain_args(c, &slots, args) {
                Ok(args) => {
                    let prof = self.prof_ctor(c);
                    let v = self.construct_plain(c, &slots, args);
                    self.prof_exit(prof);
                    return Ok(v);
                }
                Err(args) => args,
            },
            None => args,
        };
        let env = (self.capture_count(c) > 0).then(|| fr.clone());
        if let Some(env) = env.as_ref().filter(|_| self.resident) {
            registry::frame(env);
        }
        let ordinal = if kind == ClassKind::EnumCase { self.syms().class(c).ordinal as i32 } else { 0 };
        let obj = Rc::new(Object { class: c, fields: RefCell::new(Vec::new()), env, name: None, ordinal: Cell::new(ordinal), hash: Cell::new(0), slot: Cell::new(0), born: Cell::new(0), made: Cell::new(super::made_now()) });
        self.enter()?;
        let prof = self.prof_ctor(c);
        let r = self.construct(&obj, c, args);
        self.prof_exit(prof);
        self.depth -= 1;
        r?;
        Ok(Value::Obj(obj))
    }

    pub(super) fn construct_via_new(&mut self, via: SymId, args: Vec<Value>) -> R {
        let Owner::Class(c) = self.syms().sym(via).owner else { return self.unsupported("a secondary constructor without a class") };
        if let Some(companion) = self.touched_companion(c) {
            self.module(companion)?;
        }
        let obj = Rc::new(Object { class: c, fields: RefCell::new(Vec::new()), env: None, name: None, ordinal: Cell::new(0), hash: Cell::new(0), slot: Cell::new(0), born: Cell::new(0), made: Cell::new(super::made_now()) });
        self.enter()?;
        let r = self.construct_via(&obj, via, args);
        self.depth -= 1;
        r?;
        Ok(Value::Obj(obj))
    }

    /// Runs the primary constructor of `c` on `obj`: the parameters become fields, the
    /// superclass is constructed, the body runs.
    fn construct(&mut self, obj: &Rc<Object>, c: ClassId, args: Vec<Value>) -> R<()> {
        let ti = self.tclass_index(c);
        self.halt_on_withheld()?;
        let Some(ti) = ti else {
            // A class whose check is under way when a macro defined in it runs: its members are
            // typed on demand as the macro reaches them, its body has not run yet.
            if self.macro_ctx.is_some() && self.typer.check_under_way(c) {
                return Ok(());
            }
            // A JDK object is made without its constructor, which the class path has no body
            // for: a module that keeps one (scala-library's `Console` and its reader) initialises,
            // and a member the interpreter has no native for fails where it is called. The
            // arguments stay in its fields for the natives that read them (`jdk_member`).
            if self.typer.is_java_class(c) || self.typer.is_java_placeholder(c) {
                *obj.fields.borrow_mut() = args;
                return Ok(());
            }
            let name = self.class_path(c);
            return self.unsupported(format!("{} has no constructor to run", name));
        };
        let (params, defaults, parent_args, parent_via, prelude, captures) = {
            let tc = &self.prog().classes[ti];
            (tc.ctor_params.clone(), tc.ctor_defaults.clone(), tc.parent_args, tc.parent_via, tc.parent_prelude, tc.captures)
        };
        let info = self.syms().class(c);
        let (kind, superclass) = (info.kind, info.superclass.filter(|_| info.kind != ClassKind::Trait));
        let is_anon = kind == ClassKind::Anon;
        let this = Value::Obj(obj.clone());
        let key = self.fresh_key();
        let cx = Ctx { this: this.clone(), def_key: key, class: Some(c), tail: None };
        let frame = Frame::with(obj.env.clone(), Vec::with_capacity(params.len()));
        // What the class captures is read from its environment; the values that follow go to
        // the parameters of a named local class, or to the superclass of an anonymous one.
        let captures = if is_anon { params.len() } else { captures };
        let mut args = args.into_iter();
        let mut parent_vals: Option<Vec<Value>> = None;
        if is_anon {
            parent_vals = Some(args.by_ref().skip(captures).collect());
        } else {
            // A named class's captures (the outer instance of a jar's nested class) passed as
            // arguments are kept as fields, where the methods it lends a subclass read them.
            for &p in params.iter().take(captures) {
                let v = args.next().unwrap_or(Value::Absent);
                if !matches!(v, Value::Absent) {
                    frame.bind(p, v.clone());
                    let key = self.field_key(p);
                    self.bind_slot(&this, key, v);
                }
            }
            for (i, &p) in params.iter().enumerate().skip(captures) {
                let mut v = args.next().unwrap_or(Value::Absent);
                if let Value::Absent = v {
                    v = match defaults[i] {
                        Some(d) => self.eval(d, &frame, &cx)?,
                        None => Value::Unit,
                    };
                }
                frame.bind(p, v.clone());
                let key = self.field_key(p);
                self.bind_slot(&this, key, v);
            }
        }
        let frame = self.exec_stmts(prelude, &frame, &cx)?;
        match (parent_via, superclass, parent_args) {
            (Some(via), Some(_), _) => {
                let vals = match parent_vals.take() {
                    Some(v) => v,
                    None => match parent_args {
                        Some(l) => self.eval_args(Some(via), l, &frame, &cx)?,
                        None => Vec::new(),
                    },
                };
                self.construct_via(obj, via, vals)?;
            }
            (None, Some(sup), _) => {
                let vals = match parent_vals.take() {
                    Some(v) => v,
                    None => match parent_args {
                        Some(l) => self.eval_parent_args(sup, l, &frame, &cx)?,
                        None => Vec::new(),
                    },
                };
                self.construct(obj, sup, vals)?;
            }
            (_, None, Some(l)) => {
                if let Some(e) = self.enum_of_case(c) {
                    let vals = match parent_vals.take() {
                        Some(v) => v,
                        None => self.eval_parent_args(e, l, &frame, &cx)?,
                    };
                    self.construct(obj, e, vals)?;
                }
            }
            _ => {}
        }
        if kind == ClassKind::Object {
            self.create_stateful_enum_values(c, &this)?;
        }
        self.run_inits(obj, c, ti, &frame, &cx)
    }

    fn eval_parent_args(&mut self, sup: ClassId, l: ListRef, frame: &Rc<Frame>, cx: &Ctx) -> R<Vec<Value>> {
        let mut out = Vec::with_capacity(l.len as usize);
        for i in 0..l.len as usize {
            let item = self.prog().expr_lists[l.start as usize + i];
            if let TExpr::Unit = self.prog().expr(item) {
                let has_default = self.syms().class(sup).ctor.iter().flat_map(|cl| cl.params.iter()).nth(i).map_or(false, |p| p.has_default);
                if has_default {
                    out.push(Value::Absent);
                    continue;
                }
            }
            out.push(self.eval(item, frame, cx)?);
        }
        Ok(out)
    }

    /// The field initialisers, statements and trait bodies that the construction of `c` runs.
    fn run_inits(&mut self, obj: &Rc<Object>, c: ClassId, ti: usize, frame: &Rc<Frame>, cx: &Ctx) -> R<()> {
        #[cfg(debug_assertions)]
        if self.typer.forked && crate::types::view::probing() {
            let me = self.typer.worker;
            let foreign = |id: u32| id >= crate::arena::LOCAL_BASE && crate::arena::worker_of(id) != Some(me);
            let tc = &self.typer.prog.classes[ti];
            let bad: Vec<String> = tc
                .init
                .iter()
                .filter_map(|i| match i {
                    TInit::Field(s, e) if foreign(e.0) || foreign(s.0) => Some(format!("field {:?} = {:?}", s, e)),
                    TInit::Stmt(e) if foreign(e.0) => Some(format!("statement {:?}", e)),
                    _ => None,
                })
                .collect();
            if !bad.is_empty() {
                let file = self.syms().class(c).file;
                crate::types::view::probe(|| {
                    format!(
                        "worker {} runs the initialisers of class {} ({:?}, {}), class body {} ({}), which name a peer's records, read as escaped: {}",
                        me,
                        self.typer.name_str(self.syms().class(c).name),
                        c,
                        self.typer.source(file).path,
                        ti,
                        if (ti as u32) < crate::arena::LOCAL_BASE { "shared" } else { "own" },
                        bad.join(", ")
                    )
                });
            }
        }
        let this = Value::Obj(obj.clone());
        let n = self.prog().classes[ti].init.len();
        for i in 0..n {
            let init = match self.prog().classes[ti].init[i] {
                TInit::Field(s, e) => (0, s, e, None),
                TInit::Stmt(e) => (1, SymId(0), e, None),
                TInit::Parent(b, call) => (2, SymId(0), TExprId(0), Some((b, call))),
            };
            match init {
                (0, s, e, _) => {
                    let m = self.syms().sym(s).mods;
                    if m & mods::LAZY != 0 {
                        continue;
                    }
                    let key = self.field_key(s);
                    if self.syms().sym(s).overridden_by_param {
                        let v = self.eval(e, frame, cx)?;
                        if matches!(self.read_slot(&this, key), Value::Absent) {
                            self.write_slot(&this, key, v);
                        }
                        continue;
                    }
                    let v = self.eval(e, frame, cx)?;
                    self.write_slot(&this, key, v);
                }
                (1, _, e, _) => {
                    self.eval(e, frame, cx)?;
                }
                (_, _, _, Some((tr, call))) => self.trait_init(obj, c, tr, call, frame, cx)?,
                _ => unreachable!(),
            }
        }
        Ok(())
    }

    /// The body of a trait, run with what the class passes to its parameters.
    fn trait_init(&mut self, obj: &Rc<Object>, c: ClassId, tr: ClassId, call: ParentCall, frame: &Rc<Frame>, cx: &Ctx) -> R<()> {
        let ti = self.tclass_index(tr);
        self.halt_on_withheld()?;
        let Some(ti) = ti else {
            let name = self.class_path(tr);
            return self.unsupported(format!("{} has no body to run", name));
        };
        let scope = self.exec_stmts(call.prelude, frame, cx)?;
        let vals = match call.via {
            Some(via) => self.eval_args(Some(via), call.args, &scope, cx)?,
            None => self.eval_parent_args(tr, call.args, &scope, cx)?,
        };
        let params = self.prog().classes[ti].ctor_params.clone();
        let defaults = self.prog().classes[ti].ctor_defaults.clone();
        let this = Value::Obj(obj.clone());
        let tframe = Frame::with(obj.env.clone(), Vec::with_capacity(params.len()));
        let tcx = Ctx { this: this.clone(), def_key: cx.def_key, class: Some(tr), tail: None };
        let mut vals = vals.into_iter();
        for (i, &p) in params.iter().enumerate() {
            let key = self.field_key(p);
            let mut v = vals.next().unwrap_or(Value::Absent);
            if let Value::Absent = v {
                match defaults[i] {
                    Some(d) => v = self.eval(d, &tframe, &tcx)?,
                    // The evidence the class sets as a field before the trait's body runs.
                    None => {
                        let existing = self.read_slot(&this, key);
                        tframe.bind(p, if let Value::Absent = existing { Value::Unit } else { existing });
                        continue;
                    }
                }
            }
            tframe.bind(p, v.clone());
            self.write_slot(&this, key, v);
        }
        let _ = c;
        self.run_inits(obj, tr, ti, &tframe, &tcx)
    }

    /// A secondary constructor: its parameters, the statements before its self call, the
    /// constructor that call goes to, then the statements after it.
    fn construct_via(&mut self, obj: &Rc<Object>, via: SymId, args: Vec<Value>) -> R<()> {
        let Owner::Class(c) = self.syms().sym(via).owner else { return self.unsupported("a secondary constructor without a class") };
        let Some(ti) = self.tclass_index(c) else {
            if self.typer.is_java_class(c) {
                return Ok(());
            }
            let name = self.class_path(c);
            return self.unsupported(format!("a secondary constructor of {} whose class was not typed", name));
        };
        let f = self.prog().classes[ti].ctors.iter().copied().find(|&f| self.prog().funs[f.idx()].sym == via);
        let Some(f) = f else {
            let name = self.class_path(c);
            return self.unsupported(format!("a secondary constructor of {} that was not typed", name));
        };
        let this = Value::Obj(obj.clone());
        let key = self.fresh_key();
        let cx = Ctx { this: this.clone(), def_key: key, class: Some(c), tail: None };
        let frame = self.bind_ctor_params(f, args, obj.env.clone(), &cx)?;
        let Some(TExpr::Block(stmts, after)) = self.prog().funs[f.idx()].body.map(|b| self.prog().expr(b)) else {
            return self.unsupported("a secondary constructor without a self call");
        };
        let Some(TStmt::Expr(call)) = stmts.len.checked_sub(1).map(|i| self.prog().stmts[stmts.start as usize + i as usize]) else {
            return self.unsupported("a secondary constructor without a self call");
        };
        let before = ListRef { start: stmts.start, len: stmts.len - 1 };
        let scope = self.exec_stmts(before, &frame, &cx)?;
        match self.prog().expr(call) {
            TExpr::New(target, l) => {
                let vals = self.eval_parent_args(target, l, &scope, &cx)?;
                self.construct(obj, target, vals)?;
            }
            TExpr::NewVia(next, l) => {
                let vals = self.eval_args(Some(next), l, &scope, &cx)?;
                self.construct_via(obj, next, vals)?;
            }
            _ => return self.unsupported("a secondary constructor whose self call has another shape"),
        }
        self.eval(after, &scope, &cx)?;
        Ok(())
    }

    fn bind_ctor_params(&mut self, f: FunId, args: Vec<Value>, parent: Option<Rc<Frame>>, cx: &Ctx) -> R<Rc<Frame>> {
        let n = self.prog().funs[f.idx()].params.len();
        let frame = Frame::with(parent, Vec::with_capacity(n));
        let mut args = args.into_iter();
        for i in 0..n {
            let p = self.prog().funs[f.idx()].params[i];
            let mut v = args.next().unwrap_or(Value::Absent);
            if let Value::Absent = v {
                v = match self.prog().funs[f.idx()].defaults[i] {
                    Some(d) => self.eval(d, &frame, cx)?,
                    None => Value::Unit,
                };
            }
            frame.bind(p, v);
        }
        Ok(frame)
    }

    /// The values of a stateful enum are made by its companion, in declaration order, before
    /// the companion's own body runs; a value kept from an earlier build with the cacheable state
    /// that reaches it (`forget_made`) stays the one value of its case.
    fn create_stateful_enum_values(&mut self, companion: ClassId, this: &Value) -> R<()> {
        let Some(e) = self.syms().class(companion).companion else { return Ok(()) };
        let info = self.syms().class(e);
        if info.kind != ClassKind::Enum || !info.stateful {
            return Ok(());
        }
        let children = info.children.clone();
        for case in children {
            let Some(sym) = self.syms().class(case).singleton else { continue };
            let v = match self.made.statics.get(&sym) {
                Some(kept) => kept.clone(),
                None => self.make_enum_value(case, sym)?,
            };
            let key = self.field_key(sym);
            self.write_slot(this, key, v);
        }
        Ok(())
    }

    /// Whether the constructor of `c` runs statements or initialisers of the body.
    pub(super) fn has_body(&mut self, c: ClassId) -> bool {
        if let Some(b) = self.has_body[c.idx()] {
            return b;
        }
        let Some(ti) = self.tclass_index(c) else { return false };
        let inits = self.prog().classes[ti].init.iter().any(|i| match i {
            TInit::Field(s, _) => self.syms().sym(*s).mods & mods::LAZY == 0,
            TInit::Stmt(_) | TInit::Parent(..) => true,
        });
        let via = self.prog().classes[ti].parent_via;
        let b = inits || via.map_or(false, |via| self.via_has_statements(via));
        self.has_body[c.idx()] = Some(b);
        b
    }

    fn via_has_statements(&mut self, via: SymId) -> bool {
        let mut cur = via;
        let mut steps = 0;
        while steps <= self.syms().classes.len() {
            steps += 1;
            let Owner::Class(c) = self.syms().sym(cur).owner else { return false };
            let Some(ti) = self.tclass_index(c) else { return false };
            let Some(f) = self.prog().classes[ti].ctors.iter().copied().find(|&f| self.prog().funs[f.idx()].sym == cur) else { return false };
            let Some(TExpr::Block(stmts, after)) = self.prog().funs[f.idx()].body.map(|b| self.prog().expr(b)) else { return false };
            if !matches!(self.prog().expr(after), TExpr::Unit) {
                return true;
            }
            match stmts.len.checked_sub(1).map(|i| self.prog().stmts[stmts.start as usize + i as usize]) {
                Some(TStmt::Expr(call)) => match self.prog().expr(call) {
                    TExpr::NewVia(next, _) => cur = next,
                    _ => return false,
                },
                _ => return false,
            }
        }
        false
    }

    /// Initialises the companion whose body scalac's synthetic `apply` of a case class runs, once.
    pub(super) fn touch_companion(&mut self, c: ClassId) -> R<()> {
        if let Some(companion) = self.touched_companion(c) {
            if self.made.modules.get(companion.idx()).map_or(true, |m| m.is_none()) {
                self.module(companion)?;
            }
        }
        Ok(())
    }

    /// The companion whose body scalac's synthetic `apply` of a case class runs.
    fn touched_companion(&mut self, c: ClassId) -> Option<ClassId> {
        let info = self.syms().class(c);
        if info.kind != ClassKind::Class || info.mods & mods::CASE == 0 {
            return None;
        }
        let companion = info.companion?;
        (self.syms().class(companion).kind == ClassKind::Object && self.has_body(companion)).then_some(companion)
    }

    pub(super) fn array_seq_of(&mut self, array: Value) -> R {
        // The std enters `ArraySeq` when it types a varargs literal, which may be after this
        // interpreter started: a macro whose own code has the first literal of the build.
        if self.array_seq.is_none() {
            self.array_seq = self.typer.array_seq_class();
        }
        match self.array_seq {
            Some(c) => self.construct_new(c, vec![array], &Frame::new(None)),
            None => Ok(array),
        }
    }

    // ---- fields ----

    /// The slot key of a member: its output name, told apart from a same-named member of a class
    /// above or below where it is private.
    pub(super) fn field_key(&mut self, s: SymId) -> FieldKey {
        if let Some(k) = self.field_keys.get(&s) {
            return *k;
        }
        let info = self.syms().sym(s);
        let name = self.syms().dispatch_name(s);
        let mut tag = 0;
        if let Owner::Class(c) = info.owner {
            if info.mods & mods::PRIVATE != 0 && !info.scoped_private && (self.is_trait_state(c, s) || self.syms().private_name_clashes(c, info.name)) {
                tag = c.0 + 1;
            }
        }
        let key = FieldKey(name, tag);
        self.field_keys.insert(s, key);
        key
    }

    fn is_trait_state(&self, c: ClassId, s: SymId) -> bool {
        let info = self.syms().class(c);
        if info.kind != ClassKind::Trait {
            return false;
        }
        let sym = self.syms().sym(s);
        let is_field = matches!(sym.kind, SymKind::Val | SymKind::Var) && sym.def.is_some() && sym.mods & mods::LAZY == 0;
        is_field || info.ctor.iter().any(|cl| cl.params.iter().any(|p| p.sym == s))
    }

    pub(super) fn field_key_of_param(&mut self, s: SymId) -> Option<FieldKey> {
        let info = self.syms().sym(s);
        match (info.kind, info.owner) {
            (SymKind::Param, Owner::Class(_)) => Some(self.field_key(s)),
            _ => None,
        }
    }

    pub(super) fn slot(&mut self, c: ClassId, key: FieldKey) -> u32 {
        let layout = self.layouts[c.idx()].get_or_insert_with(|| Box::new(FxMap::default()));
        let n = layout.len() as u32;
        *layout.entry(key).or_insert(n)
    }

    pub(super) fn slot_if_known(&self, c: ClassId, key: FieldKey) -> Option<u32> {
        self.layouts.get(c.idx())?.as_ref()?.get(&key).copied()
    }

    /// Marks the slot of `key` in `c`'s layout as a lazy val's, which an identity hash leaves out
    /// whether or not the val was read (`identity.rs`).
    fn mark_lazy_slot(&mut self, c: ClassId, key: FieldKey) {
        self.lazy_slots[c.idx()].get_or_insert_with(|| Box::new(FxMap::default())).insert(key, ());
    }

    pub(super) fn read_slot(&mut self, obj: &Value, key: FieldKey) -> Value {
        let Value::Obj(o) = obj else { return Value::Absent };
        match self.slot_if_known(o.class, key) {
            Some(slot) => o.fields.borrow().get(slot as usize).cloned().unwrap_or(Value::Absent),
            None => Value::Absent,
        }
    }

    /// Writes a field after the object was made: an initialiser of the class body, an
    /// assignment, a lazy val's first read. A reference written so may close a cycle
    /// (`registry.rs`).
    pub(super) fn write_slot(&mut self, obj: &Value, key: FieldKey, v: Value) {
        if self.resident {
            registry::written(obj, &v);
        }
        self.bind_slot(obj, key, v)
    }

    /// Binds a field to one of the constructor's arguments, which existed before the object.
    pub(super) fn bind_slot(&mut self, obj: &Value, key: FieldKey, v: Value) {
        let Value::Obj(o) = obj else { return };
        super::written(o.born.get(), o.made.get(), || Some(super::Node::Value(obj.clone())), |_| format!("a field of an instance of {}", self.class_path(o.class)));
        let slot = self.slot(o.class, key) as usize;
        let mut fields = o.fields.borrow_mut();
        if fields.len() <= slot {
            fields.resize(slot + 1, Value::Absent);
        }
        fields[slot] = v;
    }

    pub(super) fn get_field(&mut self, recv: Value, s: SymId) -> R {
        match &recv {
            Value::Obj(o) => {
                let key = self.field_key(s);
                let v = self.read_slot(&recv, key);
                if !matches!(v, Value::Absent) {
                    return Ok(v);
                }
                let class = o.class;
                let name = self.syms().dispatch_name(s);
                let m = self.member(class, name);
                match m {
                    Member::Missing | Member::Field(_) => Ok(Value::Null),
                    m => self.invoke_member(recv, m, name, Vec::new()),
                }
            }
            Value::Null => self.throw_named("NullPointerException", &format!("Cannot read field \"{}\" because the value is null", self.name(self.syms().sym(s).name))),
            Value::Fun(_) => {
                let name = self.syms().dispatch_name(s);
                self.invoke(recv, s, Vec::new()).or_else(|e| match e {
                    Control::Fail(_) => {
                        let n = self.name(name).to_string();
                        self.unsupported(format!("the field {} of a function", n))
                    }
                    other => Err(other),
                })
            }
            _ => self.invoke(recv, s, Vec::new()),
        }
    }

    pub(super) fn set_field(&mut self, recv: Value, s: SymId, v: Value) -> R<()> {
        match &recv {
            Value::Obj(o) if self.pure && self.macro_ctx.is_none() && self.made.modules.get(o.class.idx()).map_or(false, |m| m.as_ref().map_or(false, |m| Rc::ptr_eq(m, o))) => {
                self.impure("an assignment to a field of an object")
            }
            Value::Obj(_) => {
                let key = self.field_key(s);
                self.write_slot(&recv, key, v);
                Ok(())
            }
            Value::Null => self.throw_named("NullPointerException", "Cannot assign a field because the value is null"),
            _ => self.unsupported("a field assignment on a value"),
        }
    }

    // ---- members ----

    pub(super) fn member(&mut self, c: ClassId, name: Name) -> Member {
        if let Some(m) = self.members.get(&(c, name)) {
            return m.clone();
        }
        let mut m = self.resolve_member(c, name);
        if matches!(m, Member::Missing) {
            m = self.var_setter(c, name);
        }
        self.members.insert((c, name), m.clone());
        m
    }

    /// `x_=` of a class whose instances hold the var `x`, which implements an abstract setter;
    /// an alternative of an overloaded setter goes by `x_=` and a suffix (`x_=$Int`).
    fn var_setter(&mut self, c: ClassId, name: Name) -> Member {
        let text = self.name(name);
        let var_text = text.rfind("_=").filter(|&at| text[at + 2..].is_empty() || text[at + 2..].starts_with('$')).map(|at| &text[..at]);
        let Some(var_name) = var_text.and_then(|n| self.typer.interner.lookup(n)) else { return Member::Missing };
        let interner = self.typer.interner;
        let var = self.resolution_chain(c).into_iter().find_map(|k| {
            crate::typer::setters::own_var(self.syms(), k, var_name).filter(|&v| !crate::typer::setters::is_abstract_var(self.syms(), interner, v))
        });
        match var {
            Some(var) => Member::SetField(self.field_key(var)),
            None => Member::Missing,
        }
    }

    /// The classes a member of `c` is looked up in, in the order the prototype chain answers.
    fn resolution_chain(&self, c: ClassId) -> Vec<ClassId> {
        let mut out = Vec::new();
        let mut at = Some(c);
        let mut steps = 0;
        while let Some(k) = at {
            out.extend(self.own_bases(k));
            let info = self.syms().class(k);
            at = info.superclass.filter(|_| info.kind != ClassKind::Trait);
            steps += 1;
            if steps > self.syms().classes.len() {
                break;
            }
        }
        out
    }

    fn flat_params(&self, s: SymId) -> Vec<crate::types::TypeId> {
        match &self.syms().sym(s).sig {
            Some(sig) => sig.clauses.iter().flat_map(|c| c.params.iter().map(|p| p.ty)).collect(),
            None => Vec::new(),
        }
    }

    /// Whether class `k` is scala-library's, whose overrides of generic alternatives
    /// (`List.appendedAll`, `ArraySeq.copyToArray`) specialise what the inherited
    /// implementation gives, some through JDK methods the interpreter has no native for.
    fn in_scala_package(&self, k: ClassId) -> bool {
        let mut at = self.syms().class(k).owner;
        while let Owner::Class(o) = at {
            at = self.syms().class(o).owner;
        }
        let Owner::Package(mut p) = at else { return false };
        while let Some(parent) = self.syms().pkg(p).parent.filter(|&q| q != crate::symbols::ROOT_PKG) {
            p = parent;
        }
        p == self.typer.b.scala_pkg
    }

    /// Whether `a` has the parameters of `s`, its own type parameters standing for those of
    /// `s`: the override of a generic alternative in a jar's subclass (zio's
    /// `Singleton.toArray[A1 >: A](srcPos: Int, dest: Array[A1], ..)`), which no override
    /// record names.
    fn overrides_by_signature(&mut self, a: SymId, s: SymId) -> bool {
        let (Some(sa), Some(ss)) = (self.syms().sym(a).sig.clone(), self.syms().sym(s).sig.clone()) else { return false };
        let shape = |sig: &crate::symbols::MethodSig| sig.clauses.iter().map(|c| c.params.len()).collect::<Vec<_>>();
        if sa.tparams.is_empty() || sa.tparams.len() != ss.tparams.len() || shape(&sa) != shape(&ss) {
            return false;
        }
        let mut subst: crate::types::Subst = ss.tparams.iter().copied().zip(sa.tparams.iter().map(|&t| self.typer.types.param(t)).collect::<Vec<_>>()).collect();
        // The class's own type parameters as the class of `a` sees them (`A` of `A1 >: A`).
        if let (Owner::Class(k), Owner::Class(owner)) = (self.syms().sym(a).owner, self.syms().sym(s).owner) {
            let k_tparams = self.syms().class(k).tparams.clone();
            let own: Vec<crate::types::TypeId> = k_tparams.into_iter().map(|p| self.typer.types.param(p)).collect();
            let this = self.typer.types.class(k, &own);
            if let Some(base) = self.typer.base_type(this, owner) {
                if let Type::Class(_, args) = self.typer.types.get(base) {
                    let args = self.typer.types.items(args).to_vec();
                    let owner_tparams = self.syms().class(owner).tparams.clone();
                    subst.extend(owner_tparams.into_iter().zip(args));
                }
            }
        }
        let mut pairs: Vec<(crate::types::TypeId, crate::types::TypeId)> =
            ss.clauses.iter().zip(&sa.clauses).flat_map(|(x, y)| x.params.iter().zip(&y.params).map(|(p, q)| (p.ty, q.ty)).collect::<Vec<_>>()).collect();
        // The type parameters' bounds too: `f[B <: CharSequence](x: B)` is an overload of an
        // inherited `f[A <: Number](x: A)`, not its override.
        for (&ps, &pa) in ss.tparams.iter().zip(&sa.tparams) {
            let (bs, ba) = (self.syms().tparam(ps).clone(), self.syms().tparam(pa).clone());
            pairs.push((bs.upper, ba.upper));
            pairs.push((bs.lower, ba.lower));
        }
        pairs.into_iter().all(|(ps, pa)| {
            let seen = self.typer.types.subst(ps, &subst);
            seen == pa || self.typer.is_same(seen, pa)
        })
    }

    /// Whether `a` is `s` or overrides it, through the methods in between.
    fn implements(&self, a: SymId, s: SymId, depth: u32) -> bool {
        if a == s {
            return true;
        }
        if depth > 32 {
            return false;
        }
        match self.prog().overrides.get(&a) {
            Some(bases) => bases.iter().any(|&b| self.implements(b, s, depth + 1)),
            None => false,
        }
    }

    /// The implementation of the overloaded alternative `s` for an instance of `c`: in the
    /// nearest class, the alternative that overrides `s`, or the one with its parameter types.
    fn alternative_member(&mut self, c: ClassId, s: SymId) -> Member {
        if let Some(m) = self.alt_members.get(&(c, s)) {
            return m.clone();
        }
        let name = self.syms().sym(s).name;
        let types = self.flat_params(s);
        let mut erased: Option<(String, Option<Name>)> = None;
        let mut found = Member::Missing;
        for k in self.resolution_chain(c) {
            let Some(&m) = self.syms().class(k).members.get(&name) else { continue };
            let alts: Vec<SymId> = match self.syms().alternatives(m) {
                Some(alts) => alts.to_vec(),
                None => vec![m],
            };
            // A val or var of the name overrides by its accessor, which the lookup by name finds.
            if alts.iter().any(|&a| {
                let info = self.syms().sym(a);
                info.owner == Owner::Class(k) && matches!(info.kind, SymKind::Val | SymKind::Var)
            }) {
                break;
            }
            let own: Vec<SymId> = alts
                .into_iter()
                .filter(|&a| {
                    let info = self.syms().sym(a);
                    matches!(info.kind, SymKind::Def | SymKind::Given) && !info.is_extension && info.owner == Owner::Class(k)
                })
                .collect();
            let pick = own
                .iter()
                .copied()
                .find(|&a| self.implements(a, s, 0))
                .or_else(|| own.iter().copied().find(|&a| self.flat_params(a) == types))
                .or_else(|| (!self.in_scala_package(k)).then(|| own.iter().copied().find(|&a| self.overrides_by_signature(a, s))).flatten())
                .or_else(|| {
                    // The erased parameter types, the key the output's names of alternatives are
                    // made of, completing the signatures of this name's members alone; with the
                    // same `@targetName`, or none on both, since two methods that erase alike
                    // and are named apart so (`f(List[Int])` as `ints`, `f(List[String])` as
                    // `strings`) are two methods, neither overriding the other.
                    if self.in_scala_package(k) {
                        return None;
                    }
                    let (key, target) = match &erased {
                        Some(e) => e.clone(),
                        None => {
                            let e = (self.typer.erased_signature(s, true), self.typer.target_name_of(s));
                            erased = Some(e.clone());
                            e
                        }
                    };
                    own.iter().copied().find(|&a| self.typer.erased_signature(a, true) == key && self.typer.target_name_of(a) == target)
                });
            let Some(a) = pick else { continue };
            if let Some(f) = self.fun_of(a) {
                if self.prog().funs[f.idx()].body.is_some() {
                    found = Member::Fun(f, k);
                    break;
                }
            }
        }
        if let Member::Missing = found {
            let dispatch = self.syms().dispatch_name(s);
            found = self.member(c, dispatch);
        }
        self.alt_members.insert((c, s), found.clone());
        found
    }

    pub(super) fn member_by_name(&mut self, v: &Value, name: Name) -> Member {
        match v {
            Value::Obj(o) => self.member(o.class, name),
            _ => Member::Missing,
        }
    }

    /// The bases of `c` that the superclass does not bring: `c` itself and the traits it is
    /// the first to mix in, in linearisation order.
    fn own_bases(&self, c: ClassId) -> Vec<ClassId> {
        let info = self.syms().class(c);
        let superclass = info.superclass.filter(|_| info.kind != ClassKind::Trait).map(|s| self.syms().class(s));
        let own_len = superclass.map_or(info.base_types.len(), |s| {
            let tail = info.base_types.len().saturating_sub(s.base_types.len());
            if info.base_types.get(tail).map(|&(b, _)| b) == s.base_types.first().map(|&(b, _)| b) { tail } else { 0 }
        });
        match (superclass, own_len) {
            (Some(s), 0) => {
                let inherited = |b: ClassId| s.base_types.iter().any(|&(x, _)| x == b);
                info.base_types.iter().map(|&(b, _)| b).filter(|&b| !inherited(b)).collect()
            }
            _ => info.base_types[..own_len].iter().map(|&(b, _)| b).collect(),
        }
    }

    fn resolve_member(&mut self, c: ClassId, name: Name) -> Member {
        let info = self.syms().class(c);
        let superclass = info.superclass.filter(|_| info.kind != ClassKind::Trait);
        let value_class = info.value_class && info.mods & mods::CASE == 0;
        for k in self.own_bases(c) {
            let m = self.own_member(k, name);
            if !matches!(m, Member::Missing) {
                return m;
            }
            if k == c && value_class && matches!(name, crate::names::EQUALS | crate::names::HASH_CODE) {
                let defines = self.syms().class(c).members.contains_key(&name);
                if !defines {
                    if let Some(p) = self.syms().class(c).ctor.first().and_then(|cl| cl.params.first()).map(|p| p.sym) {
                        let key = self.field_key(p);
                        return if name == crate::names::EQUALS { Member::ValueEquals(key) } else { Member::ValueHash(key) };
                    }
                }
            }
        }
        if let Some(m) = self.shape_member(c, name) {
            return m;
        }
        if let Some(sup) = superclass {
            return self.member(sup, name);
        }
        match name {
            crate::names::TO_STRING => Member::AnyToString,
            crate::names::EQUALS => Member::AnyEquals,
            crate::names::HASH_CODE => Member::AnyHash,
            _ => Member::Missing,
        }
    }

    /// The few JDK members a macro reaches on an object the interpreter made itself: a
    /// `java.nio.file.Path` from a source file's path (`SourceFile.getJPath`), whose string is
    /// its first field.
    /// `String.valueOf` of the overload the call selected: the characters of a `char[]` (from
    /// `offset`, `count` of them) for the one that takes it, which a `null` makes throw; the
    /// string form of a value for the others, an array's as the interpreter renders one (it
    /// has no kind there), `null` as `"null"`.
    fn string_value_of(&mut self, takes_chars: bool, args: &[Value]) -> R<Value> {
        match (takes_chars, args) {
            (true, [Value::Null, ..]) => self.throw_named("NullPointerException", "Cannot read the array length because \"value\" is null"),
            (true, [Value::Array(a), rest @ ..]) => {
                let chars: Vec<u16> = a.borrow().iter().map(|v| if let Value::Char(c) = v { *c } else { 0 }).collect();
                let (from, count) = match rest {
                    [Value::Int(o), Value::Int(n)] => (*o as usize, *n as usize),
                    _ => (0, chars.len()),
                };
                match chars.get(from..from + count) {
                    Some(part) => Ok(Value::string(String::from_utf16_lossy(part))),
                    None => self.throw_named("StringIndexOutOfBoundsException", &format!("offset {}, count {}, length {}", from, count, chars.len())),
                }
            }
            (false, [v]) => {
                let text = self.to_str(v)?;
                Ok(Value::string(text))
            }
            _ => self.unsupported("String.valueOf with these arguments"),
        }
    }

    fn jdk_member(&mut self, recv: &Value, name: Name, args: &[Value]) -> R<Option<Value>> {
        let Value::Obj(o) = recv else { return Ok(None) };
        if !self.typer.is_java_class(o.class) {
            return Ok(None);
        }
        let member = self.name(name).to_string();
        if self.class_path(o.class) == "java.util.ResourceBundle" {
            let key = match args.first() {
                Some(Value::Str(k)) => k.to_string(),
                _ => return Ok(None),
            };
            let fields = o.fields.borrow();
            let value = fields.chunks(2).find(|kv| matches!(&kv[0], Value::Str(k) if **k == *key)).map(|kv| kv[1].clone());
            return match member.as_str() {
                "getString" | "getObject" => match value {
                    Some(v) => Ok(Some(v)),
                    None => self.unsupported(format!("the resource bundle has no key {}", key)),
                },
                "containsKey" => Ok(Some(Value::Bool(value.is_some()))),
                _ => Ok(None),
            };
        }
        if self.class_path(o.class) != "java.nio.file.Path" {
            return Ok(None);
        }
        let path = match o.fields.borrow().first() {
            Some(Value::Str(s)) => s.to_string(),
            _ => return Ok(None),
        };
        let of = |p: String| -> R<Option<Value>> {
            let obj = Rc::new(Object { class: o.class, fields: RefCell::new(vec![Value::string(p)]), env: None, name: None, ordinal: Cell::new(0), hash: Cell::new(0), slot: Cell::new(0), born: Cell::new(0), made: Cell::new(super::made_now()) });
            Ok(Some(Value::Obj(obj)))
        };
        // The rules of std/javalib/nio.scala's bodies, Windows' on Windows (`files::path`).
        use super::files::path;
        let windows = cfg!(windows);
        match member.as_str() {
            "toString" => Ok(Some(Value::string(path))),
            "getFileName" => match path::file_name(&path, windows) {
                Some(name) => of(name.to_string()),
                None => Ok(Some(Value::Null)),
            },
            "getParent" => match path::parent(&path, windows) {
                Some(parent) => of(parent.to_string()),
                None => Ok(Some(Value::Null)),
            },
            "isAbsolute" => Ok(Some(Value::Bool(path::is_absolute(&path, windows)))),
            "toAbsolutePath" => match std::env::current_dir() {
                Ok(cwd) => of(path::absolute(&path, &path::normalized(&cwd.to_string_lossy(), windows), windows, &path::drive_dir)),
                Err(_) => of(path),
            },
            "normalize" => of(path::normalize(&path, windows)),
            "resolve" => match args.first() {
                Some(Value::Str(more)) => of(path::resolve(&path, more, windows)),
                _ => Ok(None),
            },
            _ => Ok(None),
        }
    }

    /// The `toString`, `equals` and `hashCode` a case class, case object or enum value gets by
    /// rule, unless scalac lets it inherit them.
    fn shape_member(&mut self, c: ClassId, name: Name) -> Option<Member> {
        let shape = self.shape(c);
        let bit = match name {
            crate::names::TO_STRING => 0,
            crate::names::EQUALS => 1,
            crate::names::HASH_CODE => 2,
            _ => return None,
        };
        if matches!(shape, Shape::Plain) {
            return None;
        }
        let inherited = self.tclass_field(c, |tc| tc.inherited_case_members).unwrap_or(0);
        if inherited & (1 << bit) != 0 {
            return None;
        }
        Some(match (shape, bit) {
            (_, 0) => Member::CaseToString,
            (Shape::EnumValue, 1) => Member::AnyEquals,
            (_, 1) => Member::CaseEquals,
            _ => Member::CaseHash,
        })
    }

    pub(super) fn shape(&mut self, c: ClassId) -> Shape {
        if let Some(s) = &self.shapes[c.idx()] {
            return s.clone();
        }
        let info = self.syms().class(c);
        let is_case = info.mods & mods::CASE != 0 && info.singleton.is_none();
        let name: Rc<str> = Rc::from(info.product_name(self.name(info.name)));
        let shape = if info.singleton.is_some() {
            Shape::EnumValue
        } else if is_case && (info.kind == ClassKind::Object || info.local_module.is_some() || info.inner_object.is_some()) {
            Shape::CaseObject(name)
        } else if is_case {
            let params: Vec<SymId> = info.ctor.first().map(|cl| cl.params.iter().map(|p| p.sym).collect()).unwrap_or_default();
            let keys: Vec<FieldKey> = params.into_iter().map(|p| self.field_key(p)).collect();
            let display = if self.is_tuple_name(&name) { Rc::from("") } else { name };
            Shape::Case(Rc::from(keys), display)
        } else {
            Shape::Plain
        };
        self.shapes[c.idx()] = Some(shape.clone());
        shape
    }

    pub(super) fn is_tuple_name(&self, name: &str) -> bool {
        name.strip_prefix("Tuple").map_or(false, |rest| !rest.is_empty() && rest.bytes().all(|b| b.is_ascii_digit()))
    }

    /// The case class whose fields and name an instance of `c` prints and hashes: `c` itself,
    /// or the case class it extends.
    pub(super) fn case_info(&mut self, c: ClassId) -> Option<(Rc<[FieldKey]>, Rc<str>)> {
        let mut at = Some(c);
        let mut steps = 0;
        while let Some(k) = at {
            match self.shape(k) {
                Shape::Case(fields, display) => return Some((fields, display)),
                Shape::CaseObject(name) => return Some((Rc::from(Vec::new()), name)),
                _ => {}
            }
            steps += 1;
            if steps > self.syms().classes.len() {
                break;
            }
            at = self.syms().class(k).superclass;
        }
        None
    }

    fn method_index(&mut self, k: ClassId) -> Option<usize> {
        let ti = self.tclass_index(k)?;
        let n = self.prog().classes[ti].methods.len();
        if self.method_index[k.idx()].is_none() || (self.methods_seen[k.idx()] as usize) < n {
            self.method_index[k.idx()].get_or_insert_with(|| Box::new(FxMap::default()));
            for i in self.methods_seen[k.idx()] as usize..n {
                let f = self.typer.prog.classes[ti].methods[i];
                let (sym, has_body) = {
                    let fun = &self.typer.prog.funs[f.idx()];
                    (fun.sym, fun.body.is_some())
                };
                if has_body {
                    let name = if self.syms().sym(sym).is_extension { self.extension_name(sym) } else { self.syms().dispatch_name(sym) };
                    self.method_index[k.idx()].as_mut().unwrap().insert(name, f);
                }
            }
            self.methods_seen[k.idx()] = n as u32;
        }
        Some(ti)
    }

    /// The name an extension method of a class has in the output: extensions of one class may
    /// share a name and differ by receiver or arity; the first keeps the plain name, unless a
    /// member has it, and the others get `$x1`, `$x2`, ...
    pub(super) fn extension_name(&mut self, e: SymId) -> Name {
        if let Some(&n) = self.ext_names.get(&e) {
            return n;
        }
        let Owner::Class(c) = self.syms().sym(e).owner else { return self.syms().sym(e).name };
        let extensions = self.syms().class(c).extensions.clone();
        let mut taken: FxMap<Name, u32> = FxMap::default();
        for x in extensions {
            let name = self.syms().sym(x).name;
            let k = *taken.entry(name).or_insert(self.syms().class(c).members.contains_key(&name) as u32);
            let js = if k > 0 {
                let text = format!("{}$x{}", self.name(name), k);
                self.typer.interner.intern(&text)
            } else {
                name
            };
            taken.insert(name, k + 1);
            self.ext_names.insert(x, js);
        }
        self.ext_names.get(&e).copied().unwrap_or(self.syms().sym(e).name)
    }

    /// The members of `k` by the name the output dispatches them under: the alternatives of an
    /// overloaded name and a val that answers to an inherited def included.
    fn dispatch_sym(&mut self, k: ClassId, name: Name) -> Option<SymId> {
        if self.dispatch_syms[k.idx()].is_none() {
            let mut index: FxMap<Name, SymId> = FxMap::default();
            let order = self.syms().class(k).member_order.clone();
            for m in order {
                let alts: Vec<SymId> = match self.syms().alternatives(m) {
                    Some(alts) => alts.to_vec(),
                    None => vec![m],
                };
                for alt in alts {
                    if matches!(self.syms().sym(alt).kind, SymKind::Overloaded(_)) {
                        continue;
                    }
                    index.entry(self.syms().dispatch_name(alt)).or_insert(alt);
                }
            }
            self.dispatch_syms[k.idx()] = Some(Box::new(index));
        }
        self.dispatch_syms[k.idx()].as_ref().unwrap().get(&name).copied()
    }

    /// The method named `name` that class `k` defines with a body, typed now when the library
    /// deferred it.
    fn own_method(&mut self, k: ClassId, name: Name) -> Option<FunId> {
        if self.method_index(k).is_some() {
            if let Some(&f) = self.method_index[k.idx()].as_ref().and_then(|m| m.get(&name)) {
                return Some(f);
            }
        }
        let sym = match self.dispatch_sym(k, name) {
            Some(sym) => sym,
            None => {
                let extensions = self.syms().class(k).extensions.clone();
                extensions.into_iter().find(|&e| self.extension_name(e) == name)?
            }
        };
        let info = self.syms().sym(sym);
        if !matches!(info.kind, SymKind::Def | SymKind::Given) || info.owner != Owner::Class(k) {
            return None;
        }
        let f = self.fun_of(sym)?;
        if self.prog().funs[f.idx()].body.is_none() {
            return None;
        }
        self.method_index(k);
        if let Some(index) = self.method_index.get_mut(k.idx()).and_then(|i| i.as_mut()) {
            index.insert(name, f);
        }
        Some(f)
    }

    pub(super) fn own_member(&mut self, k: ClassId, name: Name) -> Member {
        if let Some(f) = self.own_method(k, name) {
            return Member::Fun(f, k);
        }
        if let Some((b, qualified)) = self.builtin_member(k, name) {
            return Member::Builtin(b, qualified);
        }
        if let Some(ti) = self.tclass_index(k) {
            let bridges = self.prog().classes[ti].bridges.clone();
            for (declared, implementation) in bridges {
                if self.syms().dispatch_name(declared) == name {
                    let target = self.syms().dispatch_name(implementation);
                    if target != name {
                        return self.member(k, target);
                    }
                }
            }
            let forwarders = self.prog().classes[ti].forwarders.clone();
            for (member, target) in forwarders {
                if self.syms().dispatch_name(member) == name {
                    return Member::Static(target);
                }
            }
            // The implementation of a deferred given, which the class's table does not hold.
            let implemented = self.prog().classes[ti].deferred_givens.iter().find(|&&(_, s)| self.syms().dispatch_name(s) == name).map(|&(_, s)| s);
            if let Some(s) = implemented {
                if let Some(init) = self.lazy_init(s) {
                    let key = self.field_key(s);
                    return Member::Lazy(key, init, k);
                }
            }
        }
        let Some(m) = self.dispatch_sym(k, name) else { return Member::Missing };
        let (kind, lazy) = {
            let sym = self.syms().sym(m);
            // A val of a library class is a lazy getter, marked so when its class is checked.
            (sym.kind, sym.mods & mods::LAZY != 0 || (sym.kind == SymKind::Val && self.typer.is_library_member(m)))
        };
        match kind {
            SymKind::Val | SymKind::Var | SymKind::Given | SymKind::Param => {
                if lazy {
                    if let Some(init) = self.lazy_init(m) {
                        let key = self.field_key(m);
                        return Member::Lazy(key, init, k);
                    }
                }
                if kind == SymKind::Given {
                    if let Some(f) = self.fun_of(m) {
                        if self.prog().funs[f.idx()].body.is_some() {
                            return Member::Fun(f, k);
                        }
                    }
                }
                // An abstract var holds nothing: what implements it is further along.
                if kind == SymKind::Var && crate::typer::setters::is_abstract_var(self.syms(), self.typer.interner, m) {
                    return Member::Missing;
                }
                let key = self.field_key(m);
                Member::Field(key)
            }
            SymKind::Object(nested) => Member::Module(nested),
            _ => Member::Missing,
        }
    }

    /// A templated def of `k` without a body, dispatched to at run time through a parent's
    /// declaration: the builtin registered under its qualified name.
    fn builtin_member(&mut self, k: ClassId, name: Name) -> Option<(Builtin, Rc<str>)> {
        let sym = self.dispatch_sym(k, name)?;
        let info = self.syms().sym(sym);
        if !matches!(info.kind, SymKind::Def | SymKind::Given) || info.owner != Owner::Class(k) {
            return None;
        }
        if info.intrinsic.is_none() && !self.typer.is_java_class(k) {
            return None;
        }
        let qualified = self.qualified_name(sym);
        Some((self.builtins.get(&*qualified).cloned()?, qualified))
    }

    /// The initialiser of a lazy val of a class, from the `init` list of its class.
    fn lazy_init(&mut self, s: SymId) -> Option<TExprId> {
        if let Some(&(_, e)) = self.lazy_inits.get(&s) {
            crate::typer::bundle::cached_at(&self.typer.prog.exprs, e.0);
            return Some(e);
        }
        let marks = self.prog().classes.marks();
        let mut found = Vec::new();
        for (_, tc) in self.typer.prog.classes.entries_since(self.lazy_seen) {
            for init in &tc.init {
                if let TInit::Field(sym, e) = init {
                    if self.typer.syms.sym(*sym).mods & mods::LAZY != 0 {
                        found.push((*sym, (tc.id, *e)));
                    }
                }
            }
        }
        self.lazy_inits.extend(found);
        self.lazy_seen = marks;
        if let Some(&(_, e)) = self.lazy_inits.get(&s) {
            crate::typer::bundle::cached_at(&self.typer.prog.exprs, e.0);
            return Some(e);
        }
        // A library val is a lazy getter whose body the typer holds.
        let Owner::Class(c) = self.syms().sym(s).owner else { return None };
        if self.typer.is_library_class(c) {
            self.typer.check_library_class(c);
        }
        let body = self.typer.deferred_body(s);
        self.note_withheld();
        match body {
            Some(crate::typer::check::DeferredBody::Val(e)) => {
                self.lazy_inits.insert(s, (c, e));
                Some(e)
            }
            _ => {
                let e = *self.typer.val_init.get(&s)?;
                crate::typer::bundle::entered_at(&self.typer.prog.exprs, e.0, crate::typer::bundle::Entry::Val);
                Some(e)
            }
        }
    }

    pub(super) fn super_accessor(&mut self, class: ClassId, tr: ClassId, member: SymId) -> Option<Option<SymId>> {
        let ti = self.tclass_index(class)?;
        self.prog().classes[ti].super_accessors.iter().find(|a| a.of_trait == tr && a.member == member).map(|a| a.target)
    }

    // ---- invocation ----

    /// The member an instance of `class` runs for a call of `s`: an extension method and a
    /// private method are never overridden and run as named, anything else by dispatch.
    pub(super) fn object_member(&mut self, class: ClassId, s: SymId) -> (Member, Name) {
        let (direct, owner) = {
            let info = self.syms().sym(s);
            let private = info.mods & mods::PRIVATE != 0 && !info.scoped_private && matches!(info.kind, SymKind::Def);
            let owner = match info.owner {
                Owner::Class(c) => Some(c),
                _ => None,
            };
            (info.is_extension || private, owner)
        };
        // An abstract extension of a trait is implemented by the extension of that name in
        // the class, as the JavaScript output names them.
        let name = if self.syms().sym(s).is_extension { self.extension_name(s) } else { self.syms().dispatch_name(s) };
        if direct {
            if let Some(f) = self.fun_of(s) {
                if self.prog().funs[f.idx()].body.is_some() {
                    return match owner {
                        Some(k) => (Member::Fun(f, k), name),
                        None => (Member::Ext(f), name),
                    };
                }
            }
        }
        // The alternatives of an overloaded name get their output names once the whole
        // program is checked; before that, and for the sake of it, the signature tells them apart.
        // So for a method named apart from an overload below it (`Collection.remove`, which
        // `List.remove(Int)` meets, is `remove$Any`): an override of it in a class entered since
        // (a std class a macro's run reaches in a watch session's retype) may not have the name
        // yet, and settling it would complete and enter classes no walk asked for.
        let by_signature = {
            let info = self.syms().sym(s);
            info.alternative || info.dispatch == crate::symbols::Dispatch::Named
        };
        let m = if by_signature { self.alternative_member(class, s) } else { self.member(class, name) };
        if matches!(m, Member::Missing) && self.name(name) == "valueOf" && matches!(self.runtime_class_name(class).as_ref(), "scala.String$" | "java.lang.String$") {
            let takes_chars = self.syms().sym(s).sig.as_ref().and_then(|sig| sig.clauses.first()).and_then(|c| c.params.first()).map(|p| p.ty);
            let takes_chars = takes_chars.map_or(false, |t| match self.typer.types.get(t) {
                Type::Class(c, args) if c == self.typer.b.array => self.typer.types.items(args).first().copied() == Some(self.typer.b.t_char),
                _ => false,
            });
            return (Member::StringValueOf(takes_chars), name);
        }
        (m, name)
    }

    pub(super) fn invoke(&mut self, recv: Value, s: SymId, args: Vec<Value>) -> R {
        if let Value::Obj(o) = &recv {
            let class = o.class;
            let (m, name) = self.object_member(class, s);
            return self.invoke_member(recv, m, name, args);
        }
        let (direct, owner) = {
            let info = self.syms().sym(s);
            let private = info.mods & mods::PRIVATE != 0 && !info.scoped_private && matches!(info.kind, SymKind::Def);
            let owner = match info.owner {
                Owner::Class(c) => Some(c),
                _ => None,
            };
            ((info.is_extension || private) && !matches!(recv, Value::Null), owner)
        };
        if direct {
            if let Some(f) = self.fun_of(s) {
                if self.prog().funs[f.idx()].body.is_some() {
                    return self.call_fun(f, recv, args, None, owner);
                }
            }
        }
        let name = if self.syms().sym(s).is_extension { self.extension_name(s) } else { self.syms().dispatch_name(s) };
        match &recv {
            Value::Obj(_) => unreachable!(),
            Value::Fun(_) => self.invoke_function_member(recv, name, args),
            Value::Null => {
                let n = self.name(name).to_string();
                self.throw_named("NullPointerException", &format!("Cannot invoke \"{}()\" because the value is null", n))
            }
            _ => self.prim_method(recv, name, args),
        }
    }

    pub(super) fn invoke_member(&mut self, recv: Value, m: Member, name: Name, args: Vec<Value>) -> R {
        match m {
            Member::Fun(f, owner) => {
                let parent = match &recv {
                    Value::Obj(o) if owner == o.class => o.env.clone(),
                    _ => None,
                };
                self.call_fun(f, recv, args, parent, Some(owner))
            }
            Member::Ext(f) => self.call_fun(f, recv, args, None, None),
            Member::Builtin(b, qualified) => {
                let mut all = args;
                all.insert(0, recv);
                let prof = self.prof_builtin(&qualified);
                let r = b(self, &all);
                self.prof_exit(prof);
                self.recycle_vec(all);
                r
            }
            Member::Field(key) => {
                let v = self.read_slot(&recv, key);
                Ok(if let Value::Absent = v { Value::Null } else { v })
            }
            Member::SetField(key) => {
                let v = args.into_iter().next().unwrap_or(Value::Unit);
                self.write_slot(&recv, key, v);
                Ok(Value::Unit)
            }
            Member::Lazy(key, init, owner) => {
                let v = self.read_slot(&recv, key);
                if !matches!(v, Value::Absent) {
                    return Ok(v);
                }
                let env = match &recv {
                    Value::Obj(o) => o.env.clone(),
                    _ => None,
                };
                // A lazy val's initialiser is the rest of its object's making, in the object's
                // epoch (`super::watch`).
                let (outer, born) = match &recv {
                    Value::Obj(o) => (super::resume_epoch(o.born.get()), o.born.get()),
                    _ => (super::enter_epoch(), super::epoch()),
                };
                let context = super::enter_init(|| format!("the lazy val {}.{}", self.class_path(owner), self.name(key.0)));
                let frame = Frame::new(env);
                let key_id = self.fresh_key();
                let cx = Ctx { this: recv.clone(), def_key: key_id, class: Some(owner), tail: None };
                let v = self.eval(init, &frame, &cx);
                drop(context);
                if let Ok(v) = &v {
                    if let Value::Obj(o) = &recv {
                        self.mark_lazy_slot(o.class, key);
                    }
                    super::publish(|| self.write_slot(&recv, key, v.clone()));
                    super::stamp(v, born);
                }
                super::leave_epoch(outer);
                v
            }
            Member::Module(c) => self.module(c),
            Member::Static(t) => {
                if self.syms().sym(t).kind == SymKind::Def {
                    self.call_static(t, args)
                } else {
                    self.static_value(t)
                }
            }
            Member::CaseToString => Ok(Value::string(self.case_to_string(&recv)?)),
            Member::CaseEquals => {
                let other = args.into_iter().next().unwrap_or(Value::Null);
                Ok(Value::Bool(self.case_equals(&recv, &other)?))
            }
            Member::CaseHash => Ok(Value::Int(self.case_hash(&recv)?)),
            Member::ValueEquals(key) => {
                let other = args.into_iter().next().unwrap_or(Value::Null);
                let same_class = match (&recv, &other) {
                    (Value::Obj(a), Value::Obj(b)) => a.class == b.class,
                    _ => false,
                };
                if !same_class {
                    return Ok(Value::Bool(false));
                }
                let (x, y) = (self.read_slot(&recv, key), self.read_slot(&other, key));
                Ok(Value::Bool(self.equal(&x, &y)?))
            }
            Member::ValueHash(key) => {
                let x = self.read_slot(&recv, key);
                Ok(Value::Int(self.hash(&x)?))
            }
            Member::AnyToString => match self.jdk_member(&recv, name, &args)? {
                Some(v) => Ok(v),
                None => Ok(Value::string(self.any_to_string(&recv))),
            },
            Member::AnyEquals => {
                let other = args.into_iter().next().unwrap_or(Value::Null);
                Ok(Value::Bool(recv.same(&other)))
            }
            Member::AnyHash => Ok(Value::Int(self.identity_hash_of(&recv))),
            Member::StringValueOf(takes_chars) => self.string_value_of(takes_chars, &args),
            Member::Missing => {
                let class = match &recv {
                    Value::Obj(o) => self.runtime_class_name(o.class),
                    other => Rc::from(self.type_name(other)),
                };
                let n = self.name(name).to_string();
                if let Value::Obj(o) = &recv {
                    if self.typer.is_java_class(o.class) {
                        if let Some(v) = self.jdk_member(&recv, name, &args)? {
                            return Ok(v);
                        }
                        let hint = if class.starts_with("java.time.") { " (java.time: scala-java-time)" } else { "" };
                        return self.unsupported(format!("{} is a JDK class, which has no body for {} to run; put a library implementing it on the class path{}", class, n, hint));
                    }
                }
                self.unsupported(format!("{} has no member {}", class, n))
            }
        }
    }

    fn invoke_function_member(&mut self, recv: Value, name: Name, args: Vec<Value>) -> R {
        let Value::Fun(c) = &recv else { unreachable!() };
        let c = c.clone();
        match self.name(name) {
            "apply" => return self.call_closure(&c, args),
            "isDefinedAt" => {
                if let ClosureKind::Partial(_, defined) = &c.kind {
                    let defined = defined.clone();
                    return self.apply_value(defined, args);
                }
                return Ok(Value::Bool(true));
            }
            "applyOrElse" => {
                if let ClosureKind::Partial(apply_or_else, _) = &c.kind {
                    let f = apply_or_else.clone();
                    return self.apply_value(f, args);
                }
                let x = args.into_iter().next().unwrap_or(Value::Unit);
                return self.call_closure(&c, vec![x]);
            }
            "toString" => return Ok(Value::string(self.to_str(&recv)?)),
            "hashCode" => return Ok(Value::Int(self.identity_hash_of(&recv))),
            "equals" => {
                let other = args.into_iter().next().unwrap_or(Value::Null);
                return Ok(Value::Bool(recv.same(&other)));
            }
            _ => {}
        }
        if let Some(pf) = self.prog().partial_function {
            if let Member::Fun(f, owner) = self.member(pf, name) {
                return self.call_fun(f, recv, args, None, Some(owner));
            }
        }
        let n = self.name(name).to_string();
        self.unsupported(format!("a function has no member {}", n))
    }

    /// Members called on a primitive through a trait it implements: `Comparable`,
    /// `CharSequence`, and the members of `Any`.
    fn prim_method(&mut self, recv: Value, name: Name, args: Vec<Value>) -> R {
        let arg = |args: &[Value]| args.first().cloned().unwrap_or(Value::Null);
        match (self.name(name), &recv) {
            ("toString", _) => Ok(Value::string(self.to_str(&recv)?)),
            ("hashCode", _) => Ok(Value::Int(self.hash_code(&recv)?)),
            ("equals", _) => {
                let other = arg(&args);
                Ok(Value::Bool(self.equal(&recv, &other)?))
            }
            ("compareTo" | "compare", Value::Str(a)) => match arg(&args) {
                Value::Str(b) => Ok(Value::Int(compare_strings(a, &b))),
                _ => self.unsupported("compareTo of a String with a value of another kind"),
            },
            ("compareTo" | "compare", v) if v.is_number() || matches!(v, Value::Char(_)) => {
                let other = arg(&args);
                let r = if v.is_fractional() || other.is_fractional() {
                    compare_doubles(v.as_f64().unwrap_or(0.0), other.as_f64().unwrap_or(0.0))
                } else {
                    v.as_i64().unwrap_or(0).cmp(&other.as_i64().unwrap_or(0)) as i32
                };
                Ok(Value::Int(r))
            }
            ("compareTo", Value::Bool(a)) => {
                let b = arg(&args).as_bool().unwrap_or(false);
                Ok(Value::Int(if *a == b { 0 } else if *a { 1 } else { -1 }))
            }
            ("length", Value::Str(s)) => Ok(Value::Int(utf16_len(s) as i32)),
            ("charAt", Value::Str(s)) => {
                let i = arg(&args).as_i32().unwrap_or(-1);
                match if i >= 0 { char_at(s, i as usize) } else { None } {
                    Some(c) => Ok(Value::Char(c)),
                    None => self.throw_named("StringIndexOutOfBoundsException", &format!("Index {} out of bounds for length {}", i, utf16_len(s))),
                }
            }
            ("isEmpty", Value::Str(s)) => Ok(Value::Bool(s.is_empty())),
            ("intValue", v) if v.is_number() => Ok(Value::Int(v.as_i32().unwrap())),
            ("longValue", v) if v.is_number() => Ok(Value::Long(v.as_i64().unwrap())),
            ("doubleValue", v) if v.is_number() => Ok(Value::Double(v.as_f64().unwrap())),
            ("floatValue", v) if v.is_number() => Ok(Value::Float(v.as_f64().unwrap() as f32)),
            ("booleanValue", Value::Bool(b)) => Ok(Value::Bool(*b)),
            ("charValue", Value::Char(c)) => Ok(Value::Char(*c)),
            (n, v) => {
                let kind = self.type_name(v);
                self.unsupported(format!("{} has no member {}", kind, n))
            }
        }
    }

    // ---- names ----

    pub(super) fn package_path(&self, p: crate::types::PkgId) -> String {
        let info = self.syms().pkg(p);
        match info.parent {
            Some(parent) => format!("{}{}.", self.package_path(parent), self.name(info.name)),
            None => String::new(),
        }
    }

    /// `a.b.C.D`: the path of a class with dots throughout, as builtins are keyed.
    pub(super) fn class_path(&self, c: ClassId) -> String {
        let info = self.syms().class(c);
        match info.owner {
            Owner::Package(p) => format!("{}{}", self.package_path(p), self.name(info.name)),
            Owner::Class(o) => format!("{}.{}", self.class_path(o), self.name(info.name)),
            Owner::Local => self.name(info.name).to_string(),
        }
    }

    /// The name `getClass.getName` gives: packages joined with dots, the enclosing classes
    /// with `$`, an object with a `$` after it.
    pub(super) fn runtime_class_name(&self, c: ClassId) -> Rc<str> {
        let info = self.syms().class(c);
        let mut s = match info.owner {
            Owner::Package(p) => self.package_path(p),
            Owner::Class(o) => {
                let mut outer = self.runtime_class_name(o).to_string();
                if !outer.ends_with('$') {
                    outer.push('$');
                }
                outer
            }
            Owner::Local => String::new(),
        };
        s.push_str(self.name(info.name));
        if info.kind == ClassKind::Object {
            s.push('$');
        }
        Rc::from(s)
    }

    /// The qualified name a builtin is registered under: the def's path, with the simple name
    /// of the receiver's class before the name of an extension method.
    pub(super) fn qualified_name(&mut self, s: SymId) -> Rc<str> {
        if let Some(n) = self.qnames.get(&s) {
            return n.clone();
        }
        let info = self.syms().sym(s);
        let mut path = match info.owner {
            Owner::Package(p) => self.package_path(p),
            Owner::Class(c) => format!("{}.", self.class_path(c)),
            Owner::Local => String::new(),
        };
        if info.is_extension {
            let receiver = info
                .sig
                .as_ref()
                .and_then(|sig| sig.clauses.iter().take(info.ext_clauses.max(1) as usize).find(|c| !c.is_using))
                .and_then(|c| c.params.first())
                .map(|p| p.ty);
            let class_name = match receiver.map(|t| self.typer.types.get(t)) {
                Some(Type::Class(c, _)) => self.name(self.syms().class(c).name).to_string(),
                Some(Type::Param(p)) => match self.typer.types.get(self.syms().tparam(p).upper) {
                    Type::Class(c, _) => self.name(self.syms().class(c).name).to_string(),
                    _ => "Any".to_string(),
                },
                _ => "Any".to_string(),
            };
            path.push_str(&class_name);
            path.push('.');
        }
        path.push_str(self.name(info.name));
        let rc: Rc<str> = Rc::from(path);
        self.qnames.insert(s, rc.clone());
        rc
    }

    pub(super) fn type_name(&self, v: &Value) -> &'static str {
        match v {
            Value::Unit => "Unit",
            Value::Null => "Null",
            Value::Bool(_) => "Boolean",
            Value::Int(_) => "Int",
            Value::Long(_) => "Long",
            Value::Double(_) => "Double",
            Value::Float(_) => "Float",
            Value::Byte(_) => "Byte",
            Value::Short(_) => "Short",
            Value::Char(_) => "Char",
            Value::Str(_) => "String",
            Value::Obj(_) => "an object",
            Value::Fun(_) => "a function",
            Value::Array(_) => "Array",
            Value::Map(_) => "RawMap",
            Value::Trie(_) => "TrieNode",
            Value::Class(_) => "Class",
            Value::Match(_) => "a regex match",
            Value::Tree(_) => "a tree",
            Value::Type(_) => "a type",
            Value::Sym(_) => "a symbol",
            Value::Pos(..) => "a position",
            Value::Src(_) => "a source file",
            Value::Absent => "an unset value",
        }
    }

    /// A class of the standard library by its simple name, in the packages the runtime throws
    /// from: `java.lang`, `scala`, `java.util`, `scala.util.matching`, `js`.
    pub(super) fn known_class(&mut self, name: &'static str) -> Option<ClassId> {
        if let Some(c) = self.known_classes.get(name) {
            return *c;
        }
        let n = self.typer.interner.intern(name);
        let mut pkgs: Vec<crate::types::PkgId> = Vec::new();
        pkgs.extend(self.typer.java_lang_pkg());
        pkgs.push(self.typer.b.scala_pkg);
        for path in [&["scala", "util", "matching"][..], &["java", "time"][..], &["js"][..], &["scala", "runtime"][..], &["scala", "collection", "immutable"][..], &["scala", "collection", "mutable"][..], &["java", "util"][..]] {
            let mut at = Some(crate::symbols::ROOT_PKG);
            for seg in path {
                let seg_name = self.typer.interner.intern(seg);
                at = at.and_then(|p| self.syms().pkg(p).entries.get(&seg_name).and_then(|e| e.pkg));
            }
            pkgs.extend(at);
        }
        let mut found = None;
        for p in pkgs {
            let class = match self.typer.pkg_type(p, n) {
                Some(TypeRef::Class(c)) => Some(c),
                _ => match self.typer.pkg_term(p, n) {
                    Some(TermRef::Global(s)) => match self.typer.syms.sym(s).kind {
                        SymKind::Object(c) => Some(c),
                        _ => None,
                    },
                    Some(TermRef::Class(c)) => Some(c),
                    _ => None,
                },
            };
            if let Some(c) = class {
                found = Some(c);
                break;
            }
        }
        self.known_classes.insert(name, found);
        found
    }

    pub(super) fn throw_named<T>(&mut self, name: &'static str, msg: &str) -> R<T> {
        let Some(c) = self.known_class(name) else {
            return self.unsupported(format!("{}: {} (the class is not in the program)", name, msg));
        };
        if self.trace {
            eprintln!("  throw {}: {}", name, msg);
        }
        let m = if msg.is_empty() { Value::Null } else { Value::str(msg) };
        let e = self.construct_new(c, vec![m], &Frame::new(None))?;
        Err(Control::Throw(e))
    }

    /// Throws a new instance of the std class at `path`, its constructor given `args`: an
    /// exception outside the packages `throw_named` looks in (`java.nio.file`'s).
    pub(super) fn throw_new<T>(&mut self, path: &[&str], args: Vec<Value>) -> R<T> {
        let Some(c) = self.typer.class_at(path) else {
            return self.unsupported(format!("{} (the class is not in the program)", path.join(".")));
        };
        if self.trace {
            eprintln!("  throw {}", path.join("."));
        }
        let e = self.construct_new(c, args, &Frame::new(None))?;
        Err(Control::Throw(e))
    }

    pub(super) fn match_error<T>(&mut self, v: Value) -> R<T> {
        let Some(c) = self.known_class("MatchError") else {
            let shown = self.to_str(&v)?;
            return self.unsupported(format!("MatchError: {}", shown));
        };
        let e = self.construct_new(c, vec![v], &Frame::new(None))?;
        Err(Control::Throw(e))
    }
}
