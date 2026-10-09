//! The right-hand sides of a pickle's definitions: who produces each (`Producer`), and the
//! census of what was written and what stays `ELIDED` (`TEQ_BODIES_CENSUS`).

use super::*;
use crate::tir::{FunId, TExprId, TInit};

/// Where a definition's body comes from, or why it stays `ELIDED` under this stage.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(super) enum Producer {
    /// The typed IR of a source body with the capture's records.
    Source,
    /// The method of a SAM conversion, made without a definition: its body is the lambda's.
    SamMethod,
    /// A default getter, the parameter's default as typed (`TFun::defaults`,
    /// `TClass::ctor_defaults`, `InlineDefinition::defaults`).
    Default,
    /// A body scalac's `SyntheticMembers`, `Desugar` or `Namer` makes from the symbols alone,
    /// with the fresh definitions it holds.
    Synthesized(&'static str),
    /// An object's module val: `new X$()`.
    ModuleVal,
    /// What an `export` forwards to, applied to the forwarder's parameters.
    Forwarder,
    /// An inline method's body as the definition check stored it (`InlineDefinition`).
    InlineBody,
    /// A macro's body as the definition check stored it: the splice of its context function.
    MacroBody,
    /// Withheld by this stage.
    Elided(Withheld),
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(super) enum Withheld {
    InlineMethod,
    Macro,
    /// A body that holds a quote or a splice.
    Quote,
    /// A default of an inline method whose definition check held the body back.
    HeldDefault,
    /// The body of a definition the writer finds no typed body for.
    Untyped,
    /// A body holding a class whose members have no typed body, which a program's own macro
    /// makes in its expansion: the whole definition, never a written expansion of `ELIDED`
    /// members.
    MacroClass,
    /// A body that refers to a mirror teq's typer derives as a top-level val, where scalac's
    /// mirror is the class's companion.
    DerivedMirror,
}

impl Producer {
    pub(super) fn name(self) -> &'static str {
        match self {
            Producer::Source => "source body",
            Producer::SamMethod => "SAM method",
            Producer::Default => "default getter",
            Producer::Synthesized(what) => what,
            Producer::ModuleVal => "module val",
            Producer::Forwarder => "export forwarder",
            Producer::InlineBody => "inline method body",
            Producer::MacroBody => "macro body",
            Producer::Elided(w) => match w {
                Withheld::InlineMethod => "elided: inline method",
                Withheld::Macro => "elided: macro",
                Withheld::Quote => "elided: quote or splice",
                Withheld::HeldDefault => "elided: default of a held inline body",
                Withheld::MacroClass => "elided: a macro-made class's members, part 3's",
                Withheld::Untyped => "elided: no typed body",
                Withheld::DerivedMirror => "elided: a mirror teq derives",
            },
        }
    }
}

/// A definition's typed body: a method's function or a val's initialiser.
#[derive(Clone, Copy)]
pub(super) enum Root {
    Fun(FunId),
    Init(TExprId),
}

impl<'w, 'a> P<'w, 'a> {
    pub(super) fn count_body(&mut self, p: Producer) {
        self.count(p.name().to_string());
    }

    /// One more of the census's `key`, forgotten with the body being written if it is rolled
    /// back (a local definition of a withheld body is not in the pickle).
    pub(super) fn count(&mut self, key: String) {
        *self.bodies.entry(key.clone()).or_insert(0) += 1;
        if self.bodies_open > 0 {
            self.counted.push(key);
        }
    }

    /// A body rolled back: withheld by this stage's rule, or one the encoder cannot state yet,
    /// by the construct it met first.
    pub(super) fn count_withheld(&mut self, reason: &str) {
        let key = if reason.starts_with("elided: ") { reason.to_string() } else { format!("elided: unstated {}", reason) };
        self.pending_withheld = Some(key.clone());
        self.count(key);
    }

    /// Withholds the body being written, for a reason of this stage's rule.
    pub(super) fn withhold(&mut self, w: Withheld) {
        self.fail(Producer::Elided(w).name().to_string());
    }

    /// The typed body of a member, a top-level definition or a given: a method or a field of a
    /// class an expansion copied is among its class's functions or initialisers alone
    /// (`TClass::methods`, `TClass::init`).
    pub(super) fn root_of(&self, s: SymId) -> Option<Root> {
        if let Some(&f) = self.w.fun_of_sym.get(&s) {
            return Some(Root::Fun(f));
        }
        if let Some(e) = self.w.val_init.get(&s).copied() {
            return Some(Root::Init(e));
        }
        let Owner::Class(c) = self.w.syms.sym(s).owner else { return None };
        let tc = &self.w.prog.classes[*self.index.tclasses.get(&c)? as usize];
        if let Some(f) = tc.methods.iter().copied().find(|f| self.w.prog.funs[f.idx()].sym == s) {
            return Some(Root::Fun(f));
        }
        tc.init.iter().find_map(|i| match *i {
            TInit::Field(f, e) if f == s => Some(Root::Init(e)),
            _ => None,
        })
    }

    /// Who writes the right-hand side of the member `s`, a source definition.
    pub(super) fn member_producer(&mut self, s: SymId) -> Producer {
        let info = self.w.syms.sym(s);
        if info.mods & mods::INLINE != 0 && info.kind != SymKind::Val {
            let is_macro = info.def.map_or(false, |d| match &self.w.ast(info.file).def(d).kind {
                DefKind::Fun(f) => f.body.map_or(false, |b| self.w.is_macro_body(info.file, b)),
                _ => false,
            });
            let stored = self.w.inline_definitions.get(&s).map_or(false, |d| d.state == crate::tir::DefinitionState::Checked && d.body.is_some());
            return match (is_macro, stored) {
                (true, true) => Producer::MacroBody,
                (true, false) => Producer::Elided(Withheld::Macro),
                (false, true) => Producer::InlineBody,
                (false, false) => Producer::Elided(Withheld::InlineMethod),
            };
        }
        if info.def.is_none() && self.sam_method(s) {
            return Producer::SamMethod;
        }
        if self.js_native_body(s) {
            return Producer::Synthesized("js.native");
        }
        match self.root_of(s) {
            Some(_) => Producer::Source,
            None => {
                Producer::Elided(Withheld::Untyped)
            }
        }
    }

    /// Whether a definition's body is `js.native`, a facade member's, which the typer leaves
    /// untyped.
    fn js_native_body(&self, s: SymId) -> bool {
        let info = self.w.syms.sym(s);
        let Some(d) = info.def else { return false };
        let ast = self.w.ast(info.file);
        let body = match &ast.def(d).kind {
            DefKind::Fun(f) => f.body,
            DefKind::Val { rhs, .. } => *rhs,
            _ => None,
        };
        body.map_or(false, |e| matches!(ast.expr(e), crate::ast::Expr::Ident(n) | crate::ast::Expr::Select(_, n) if n == crate::names::NATIVE))
    }

    fn sam_method(&self, s: SymId) -> bool {
        matches!(self.w.syms.sym(s).owner, Owner::Class(c) if self.w.sam_classes.contains_key(&c))
    }

    /// The executable roots of a class's template other than its members and statements: the
    /// parent constructor call and the secondary constructors.
    pub(super) fn count_template_roots(&mut self, c: ClassId) {
        let Some(&i) = self.index.tclasses.get(&c) else { return };
        let tc = &self.w.prog.classes[i as usize];
        let ctors = tc.ctors.len() as u32;
        // Every class's first parent is a constructor call, `Object()` where none is written.
        let parent_call = (self.w.syms.class(c).kind != ClassKind::Trait) as u32;
        let with_args = tc.parent_args.map_or(false, |l| l.len > 0) as u32;
        let trait_args = tc.init.iter().filter(|x| matches!(x, crate::tir::TInit::Parent(_, pc) if pc.args.len > 0)).count() as u32;
        for (k, n) in [("secondary constructor", ctors), ("parent constructor call", parent_call), ("parent constructor call with arguments", with_args), ("trait parent call with arguments", trait_args)] {
            // A class of a withheld body leaves the census with it.
            *self.bodies.entry(k.to_string()).or_insert(0) += n;
            if self.bodies_open > 0 {
                self.counted.extend(std::iter::repeat(k.to_string()).take(n as usize));
            }
        }
    }

    pub(super) fn is_library_sym(&self, s: SymId) -> bool {
        let file = self.w.syms.sym(s).file;
        self.w.in_jar(file) || self.w.files.as_slice().get(file.0 as usize).map_or(true, |f| f.is_std)
    }
}
