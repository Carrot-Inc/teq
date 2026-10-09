//! The annotations a source writes on its definitions, pickled as scalac's `TreePickler` writes
//! a symbol's (`ANNOTATION tycon tree`): the tree the typer kept for each
//! (`Capture::annotations`), on the symbols scalac's `PostTyper` keeps it on by the meta
//! annotations of its class, last written first, one tree for the symbols of one annotation.

use super::*;
use crate::ast::Annot;
use crate::tir::{TExpr, TExprId};
use crate::typer::loader::{meta_annotation, META_ALL, META_COMPANION_CLASS, META_COMPANION_METHOD, META_FIELD, META_GETTER, META_NON_BEAN, META_PARAM, META_SETTER};

/// The symbol an annotation is written on, which `PostTyper` keeps it on by the meta annotations
/// of its class (`keepAnnotationsCarrying`).
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum Dest {
    /// A parameter of a method or a constructor: `@param`, or none of the metas.
    Param,
    /// A class parameter's field: `@getter` or `@field`.
    Accessor,
    /// A val, var, lazy val or given val: `@getter` or `@field`, or none of the metas.
    Field,
    /// A var's setter: `@setter`.
    Setter,
    /// The class a given with parameters defines: `@companionClass`, or none of the metas.
    GivenClass,
    /// The method of such a given: `@companionMethod`.
    GivenMethod,
    /// A method, a class, an object, a type or a type parameter: every annotation.
    Any,
}

impl<'w, 'a> P<'w, 'a> {
    /// The annotations of `annots` (written in `file`) that `dest` keeps, last written first, as
    /// scalac's symbol holds them; `fallback` writes one the typer kept no tree for.
    pub(super) fn source_annotations(&mut self, file: FileId, annots: &[Annot], dest: Dest, fallback: &mut dyn FnMut(&mut Self, &Annot)) {
        // A local definition's are not kept.
        if self.bodies_open > 0 {
            return;
        }
        for a in annots.iter().rev() {
            self.source_annotation(file, a, dest, fallback);
        }
    }

    fn source_annotation(&mut self, file: FileId, a: &Annot, dest: Dest, fallback: &mut dyn FnMut(&mut Self, &Annot)) {
        let key = (file, a.instance);
        let Some(capture) = self.w.prog.capture.as_deref() else { return fallback(self, a) };
        // A withheld annotation is counted once, whatever symbols it stands on.
        let withheld = |p: &mut Self, reason: &str| {
            if p.annotations_written.insert(key, None).is_none() {
                p.count(format!("annotation withheld: {}", reason));
            }
        };
        let Some(te) = capture.annotations.get(&key).copied() else { return withheld(self, "its `new` does not type") };
        let Some(class) = self.annotation_class(te) else { return withheld(self, "a tree of no class") };
        if !self.keeps(class, dest) {
            return;
        }
        match self.annotations_written.get(&key).copied() {
            Some(Some((tycon, tree))) => {
                self.buf.byte(ANNOTATION);
                let len = self.buf.begin_length();
                self.shared_type_at(tycon);
                self.buf.byte(SHAREDTERM);
                self.buf.reference(tree);
                self.buf.end_length(len);
                self.count("annotation".to_string());
                return;
            }
            Some(None) => return,
            None => {}
        }
        let mark = self.body_begin();
        self.buf.byte(ANNOTATION);
        let len = self.buf.begin_length();
        let tycon = self.buf.addr();
        self.class_typeref(class);
        let tree = self.buf.addr();
        let outer = std::mem::replace(&mut self.in_annotation, true);
        self.term(te);
        self.in_annotation = outer;
        self.buf.end_length(len);
        let written = match self.body_end(mark) {
            Ok(()) => {
                self.count("annotation".to_string());
                Some((tycon, tree))
            }
            Err(reason) => {
                self.count(format!("annotation withheld: {}", reason));
                None
            }
        };
        self.annotations_written.insert(key, written);
    }

    /// The class an annotation's tree instantiates.
    fn annotation_class(&self, te: TExprId) -> Option<ClassId> {
        match self.w.prog.expr(te) {
            TExpr::New(c, _) => Some(c),
            TExpr::NewVia(s, _) => match self.w.syms.sym(s).owner {
                Owner::Class(c) => Some(c),
                _ => None,
            },
            TExpr::Block(_, e) => self.annotation_class(e),
            _ => None,
        }
    }

    /// Whether `PostTyper` keeps an annotation of `class` on a symbol of `dest`
    /// (`hasOneOfMetaAnnotation`).
    fn keeps(&mut self, class: ClassId, dest: Dest) -> bool {
        let meta = |p: &mut Self| p.meta(class);
        match dest {
            Dest::Any => true,
            Dest::Param => {
                let m = meta(self);
                m & META_PARAM != 0 || m & META_NON_BEAN == 0 || self.is_scala_annotation(class, "unused")
            }
            Dest::Accessor => meta(self) & (META_GETTER | META_FIELD) != 0 || ["publicInBinary", "use", "consume"].iter().any(|n| self.is_scala_annotation(class, n)),
            Dest::Field => {
                let m = meta(self);
                m & (META_GETTER | META_FIELD) != 0 || m & META_NON_BEAN == 0
            }
            Dest::Setter => meta(self) & META_SETTER != 0,
            Dest::GivenClass => {
                let m = meta(self);
                m & META_COMPANION_CLASS != 0 || m & META_ALL == 0
            }
            Dest::GivenMethod => meta(self) & META_COMPANION_METHOD != 0,
        }
    }

    fn is_scala_annotation(&self, class: ClassId, name: &str) -> bool {
        let info = self.w.syms.class(class);
        self.w.interner.get(info.name) == name && matches!(info.owner, Owner::Package(p) if self.pkg_path(p) == "scala.annotation")
    }

    /// The meta annotations `class` carries: a source class's as its definition writes them,
    /// a library class's as its pickle holds them.
    fn meta(&mut self, class: ClassId) -> u8 {
        if let Some(&m) = self.annotation_metas.get(&class) {
            return m;
        }
        let info = self.w.syms.class(class);
        let (file, def) = (info.file, info.def);
        let m = match (self.w.loaded_meta_annotations(class), def) {
            (Some(m), _) => m,
            (None, Some(d)) => {
                let annots = self.w.ast(file).def(d).annots.clone();
                // The std's own classes are named by their meta annotations' names, which no
                // program class shadows there; a program's by the classes they resolve to.
                let std = self.w.files.as_slice().get(file.0 as usize).map_or(false, |f| f.is_std);
                let mut m = 0;
                for b in &annots {
                    let typed = self.w.prog.capture.as_deref().and_then(|c| c.annotations.get(&(file, b.instance)).copied());
                    m |= match typed.and_then(|te| self.annotation_class(te)) {
                        Some(k) => {
                            let ki = self.w.syms.class(k);
                            let in_meta = matches!(ki.owner, Owner::Package(p) if self.pkg_path(p) == "scala.annotation.meta");
                            if in_meta {
                                meta_annotation(self.w.interner.get(ki.name))
                            } else {
                                0
                            }
                        }
                        None if std => meta_annotation(self.w.interner.get(b.name)),
                        None => 0,
                    };
                }
                m
            }
            (None, None) => 0,
        };
        self.annotation_metas.insert(class, m);
        m
    }
}
