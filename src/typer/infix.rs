//! dotty's `Checking.checkValidInfix`, which `Typer.typedInfixOp` calls on each infix operation the
//! parser kept (`InfixOp`): an expression `l op r`, a type `A op B` and an extractor pattern `a op b`,
//! the operator an alphanumeric identifier, not backquoted, whose symbol (the method the application
//! calls, recorded before an eta-expansion or an inline expansion of it; the class, alias, type member or
//! type parameter a type names; the extractor) is not declared `infix`, is no member of a Scala 2 class
//! and was not compiled before 3.4, and whose right operand is not written as a block or a match, is
//! reported by the unit's version (`MigrationVersion.AlphanumericInfix`, a warning from 3.4). The
//! operator's name is tested first, which is all an operator like `+` costs; the rest is out of line.

use super::Worker;
use crate::ast::{mods, ExprId, TyExprId};
use crate::intern::Name;
use crate::source::Span;
use crate::symbols::Owner;
use crate::types::{ClassId, SymId};

/// dotty's `NameOps.isOperatorName`: operator characters at the end, before them nothing or
/// identifier characters ending in `_` (`a_+`). A character past ASCII that is no letter or digit
/// stands for Unicode's math and other symbols (`Chars.isOperatorPart`), which an identifier holds.
pub fn is_operator_name(text: &str) -> bool {
    fn operator_part(c: char) -> bool {
        if c.is_ascii() { crate::lexer::is_op_char(c as u8) } else { !c.is_alphanumeric() }
    }
    let mut chars = text.char_indices().rev().peekable();
    let mut ops = 0;
    while chars.next_if(|&(_, c)| operator_part(c)).is_some() {
        ops += 1;
    }
    match chars.next() {
        None => ops > 0 || text.is_empty(),
        Some((_, '_')) => chars.all(|(_, c)| c == '_' || c == '$' || c.is_alphanumeric()),
        Some(_) => false,
    }
}

/// `Checking.infixOKSinceFollowedBy`: dotty's parser leaves the right operand `r` a `Block` or a `Match`. Ours keeps
/// a record of each block and parentheses it collapses (`Ast::written_arg`): the outermost wrapper of `r` is a block
/// where the record's two spans are the one written (`push_written` of a block), parentheses (dotty's `Parens`)
/// where they are the parentheses and what they enclose. A block in braces of one expression is so recorded, and an
/// indented region of one expression is not, which `blockExpr` simplifies to the expression (`simplify = in.token ==
/// INDENT`). Without a wrapper, `r`'s own node: a block of several statements, braced or indented, a match (`x.match`),
/// a `{ case ... }` literal (`Match(EmptyTree, cases)`).
fn block_or_match(ast: &crate::ast::Ast, r: ExprId) -> bool {
    use crate::ast::Expr;
    match ast.written_arg(r) {
        Some((written, inner)) => written == inner,
        None => match ast.expr(r) {
            Expr::Block(_) | Expr::Match(..) => true,
            Expr::Lambda(params, _) => matches!(ast.lambda_params[params.range()], [p] if p.cases),
            _ => false,
        },
    }
}

/// What an infix operation applies (`stripBlock(result).symbol`): a method, a class or an alias.
#[derive(Clone, Copy)]
pub(super) enum Infixed {
    Method(SymId),
    Class(ClassId),
    Alias(crate::types::AliasId),
    /// A case class's pattern, through its companion's `unapply`, whose owner the companion is.
    CaseClass(ClassId),
    /// A type parameter, which no modifier declares `infix`, of the program's definition.
    TypeParam,
    /// A std class reached as the alias of scala-library's `scala` package object or `Predef` it stands for at
    /// package `scala` (`Map`, `Either`), whose owner is that Scala 2 object.
    StdAlias,
    /// A universal member (`Any`'s, `AnyRef`'s), which teq types apart from any symbol: Definitions enters `Object_eq`
    /// and `Object_ne` `Final | Infix` (Definitions.scala 433-434), `Any_equals` (299) and `Object_synchronized` (435)
    /// without `Infix`, on classes no Scala 2 library declares.
    Universal(Name),
    /// dotty's `NoSymbol`, the symbol of an application the watch recorded none for: a polymorphic function value's
    /// `apply`, its refinement's (`JointRefDenotation(NoSymbol, ..)`, `PolyFunction` declaring none), or an
    /// application that failed. Not `infix`, of no owner and no TASTy, so no exemption holds of it.
    NoSymbol,
}

/// An enclosing infix operation's watch, which `watch_infix` sets aside and `unwatch_infix` restores.
pub(super) struct InfixWatch {
    at: u32,
    called: Option<SymId>,
    passed: Option<(SymId, Option<SymId>)>,
    universal: Option<Name>,
}

/// The kind of infix operation, which the message names with the syntax to use instead.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum InfixKind {
    Method,
    Type,
    Extractor,
}

impl<'a> Worker<'a> {
    /// The operation `l op r` about to be typed, its operator ending at `op_end`: where `op` may be an
    /// alphanumeric one, its application is watched for the method it calls (`Worker::infix_watch`),
    /// the enclosing operation's watch given back for `unwatch_infix` to restore.
    #[inline(always)]
    pub(super) fn watch_infix(&mut self, op: Name, op_end: u32) -> Option<InfixWatch> {
        // Only a file with an identifier for an operator has one to check (`Ast::alpha_infix`), and a
        // name that starts with an operator character is all of them, as the lexer reads one unless
        // backquoted (which the check exempts): `+`, `::`, `<=`.
        if !self.cur_ast().alpha_infix || op_end == crate::ast::NO_END {
            return None;
        }
        let text = self.name_ref(op).as_bytes();
        if matches!(text.first(), Some(b'!' | b'#' | b'%' | b'&' | b'*' | b'+' | b'-' | b'/' | b':' | b'<' | b'=' | b'>' | b'?' | b'@' | b'\\' | b'^' | b'|' | b'~')) {
            return None;
        }
        Some(InfixWatch {
            at: std::mem::replace(&mut self.infix_watch, op_end),
            called: self.infix_called.take(),
            passed: self.infix_passed.take(),
            universal: self.infix_universal.take(),
        })
    }

    /// The watch `watch_infix` began ended, the enclosing one's restored, and what the operation is checked by: the
    /// method the application called, recorded before any expansion of it, but the `apply` of its result where it
    /// passed the operand's list on (`Worker::infix_passed`); a universal member it applied; else `NoSymbol`.
    #[inline(always)]
    pub(super) fn unwatch_infix(&mut self, outer: InfixWatch) -> Infixed {
        self.infix_watch = outer.at;
        let called = std::mem::replace(&mut self.infix_called, outer.called);
        let universal = std::mem::replace(&mut self.infix_universal, outer.universal);
        let called = match std::mem::replace(&mut self.infix_passed, outer.passed) {
            Some((by, inner)) if called == Some(by) => inner,
            _ => called,
        };
        match (called, universal) {
            (Some(s), _) => Infixed::Method(s),
            (None, Some(n)) => Infixed::Universal(n),
            (None, None) => Infixed::NoSymbol,
        }
    }

    /// The expression `l op r` typed (`Typer.typedInfixOp`), its operator ending at `op_end`, checked by
    /// `of`, what its application called (`unwatch_infix`), `NoSymbol` where it recorded none (`checkValidInfix` of
    /// `stripBlock(result).symbol`, whatever that is).
    #[cold]
    #[inline(never)]
    pub(super) fn check_infix_expr(&mut self, op: Name, r: ExprId, op_end: u32, of: Infixed) {
        if is_operator_name(self.name_ref(op)) {
            return;
        }
        let file = self.env.file;
        if !self.program_source(file) || self.inline.depth > 0 {
            return;
        }
        if block_or_match(self.cur_ast(), r) {
            return;
        }
        let len = self.name_ref(op).len() as u32;
        let at = Span::new(op_end.saturating_sub(len), op_end);
        self.check_valid_infix(InfixKind::Method, op, at, of);
    }

    /// The infix type `A op B` whose operator is the name `op_ty`, resolved to `of`.
    #[cold]
    #[inline(never)]
    pub(super) fn check_infix_type(&mut self, op_ty: TyExprId, op: Name, of: Infixed) {
        if is_operator_name(self.name_ref(op)) || !self.program_source(self.env.file) || self.inline.depth > 0 {
            return;
        }
        let at = self.cur_ast().ty_spans[op_ty.idx()];
        self.check_valid_infix(InfixKind::Type, op, at, of);
    }

    /// The infix extractor pattern `a op b` whose operator is the path `path`, matched by `of`.
    #[cold]
    #[inline(never)]
    pub(super) fn check_infix_pattern(&mut self, path: ExprId, op: Name, of: Infixed) {
        if is_operator_name(self.name_ref(op)) || !self.program_source(self.env.file) || self.inline.depth > 0 {
            return;
        }
        let at = self.cur_ast().expr_span(path);
        self.check_valid_infix(InfixKind::Extractor, op, at, of);
    }

    /// `Checking.checkValidInfix` of an alphanumeric operator at `at` (its span as written).
    fn check_valid_infix(&mut self, kind: InfixKind, op: Name, at: Span, of: Infixed) {
        // `untpd.isBackquoted`: the operator's token the scanner read backquoted (`SyntaxNotes::backquoted_idents`).
        if self.cur_ast().notes.backquoted_idents.binary_search(&at.end).is_ok() {
            return;
        }
        if self.declared_infix(of) || self.scala2_owner(of) || self.compiled_before_3_4(of) {
            return;
        }
        if self.error_covers(at) {
            return;
        }
        let name = self.name_str(op);
        let (word, alternative) = match kind {
            InfixKind::Type => ("type", format!("prefix syntax {}[...]", name)),
            InfixKind::Extractor => ("extractor", format!("prefix syntax {}(...)", name)),
            InfixKind::Method => ("method", format!("method syntax .{}(...)", name)),
        };
        let msg = format!(
            "Alphanumeric {} {} is not declared infix; it should not be used as infix operator.\nInstead, use {} or backticked identifier `{}`.{}",
            word,
            name,
            alternative,
            name,
            crate::source_version::rewrite_notice("The latter", "3.4-migration")
        );
        self.migration(crate::source_version::Migration::AlphanumericInfix, at, msg, crate::warnings::NO_ID);
    }

    /// The reporter's hiding (`UniqueMessagePositions`, 11-37): a diagnostic at an offset that an earlier one of at least
    /// its level covers is not shown, so an error over the operator hides the operation's error and its warning
    /// (`c nope 1`'s E008 at the selection; `unknown f 1`'s E006 at the operand does not). An earlier error alone: a
    /// warning teq reports over an operator before this one, a deprecation at the reference, is dotty's
    /// `CrossVersionChecks`', a later phase's, which this one hides there. A dependent error is never presented.
    fn error_covers(&self, at: Span) -> bool {
        let file = self.env.file;
        self.diags.items.iter().any(|d| !d.is_warning && !d.dependent && d.file == file && d.span.start <= at.end && at.start <= d.span.end)
    }

    /// `isDeclaredInfix`: the symbol declared `infix`, or an `unapply` of the companion of a case
    /// class declared so (a case class pattern's class).
    fn declared_infix(&self, of: Infixed) -> bool {
        match of {
            Infixed::Method(s) => {
                let info = self.syms.sym(s);
                info.mods & mods::INFIX != 0
                    || matches!(info.name, crate::names::UNAPPLY | crate::names::UNAPPLY_SEQ)
                        && match info.owner {
                            Owner::Class(o) => self.companion_class(o).is_some_and(|c| self.syms.class(c).mods & (mods::CASE | mods::INFIX) == mods::CASE | mods::INFIX),
                            _ => false,
                        }
            }
            Infixed::Class(c) | Infixed::CaseClass(c) => self.syms.class(c).mods & mods::INFIX != 0,
            Infixed::StdAlias => false,
            // A library's by its pickle's flag.
            Infixed::Alias(a) => match self.syms.alias(a).def {
                Some(d) => self.ast(self.syms.alias(a).file).def(d).mods & mods::INFIX != 0,
                None => self.loaded.as_ref().is_some_and(|l| l.infix_aliases.contains_key(&a)),
            },
            Infixed::Universal(n) => matches!(n, crate::names::EQ | crate::names::NE),
            Infixed::TypeParam | Infixed::NoSymbol => false,
        }
    }

    /// `meth.maybeOwner.is(Scala2x)`: a member of a class read from a Scala 2 library (scala-library's
    /// TASTy, whose attribute says so, or a Scala 2 jar's pickles), or of the std's class standing for
    /// one (`is_std_scala2_class`), but not the std's stand-ins for the JDK, which are Java's. A type
    /// the std defines at its package for one of scala-library's stands for the alias of `scala`'s
    /// package object or `Predef` that a name of it reaches (`type Either = scala.util.Either`), whose
    /// owner is such a class.
    fn scala2_owner(&mut self, of: Infixed) -> bool {
        let owner = match of {
            Infixed::Method(s) => {
                if self.syms.sym(s).java_defined {
                    return false;
                }
                if self.is_std_member(s) {
                    return self.std_scala2_member(s);
                }
                self.syms.sym(s).owner
            }
            // A top-level class's owner is its package, never Scala 2's.
            Infixed::Class(c) => self.syms.class(c).owner,
            Infixed::StdAlias => return true,
            Infixed::Alias(a) => self.syms.alias(a).owner,
            Infixed::CaseClass(c) => return self.is_scala2_class(c) || self.is_std_scala2_class(c),
            Infixed::TypeParam | Infixed::Universal(_) | Infixed::NoSymbol => return false,
        };
        match owner {
            Owner::Class(c) => self.is_scala2_class(c) || self.is_std_scala2_class(c),
            _ => false,
        }
    }

    /// Whether a member of the std stands for one of a Scala 2 class of scala-library: none of the Java
    /// platform's layer (`std/javalib`, `java.lang.String`'s members among them), a class's member where
    /// the class stands for such a class (`is_std_scala2_class`), and an extension at a `scala` package's
    /// top level outside the Scala 3 library's packages, which stands for a method of a wrapper of
    /// scala-library's (`RichInt.max`, `StringOps.take`).
    fn std_scala2_member(&mut self, s: SymId) -> bool {
        let file = self.syms.sym(s).file;
        if self.source(file).path.contains("javalib/") {
            return false;
        }
        match self.syms.sym(s).owner {
            Owner::Class(c) => self.is_std_scala2_class(c),
            Owner::Package(p) => {
                let path = self.pkg_path(p);
                (path == "scala" || path.starts_with("scala.")) && !super::loader::compile::in_scala3_library_package(&path)
            }
            Owner::Local => false,
        }
    }

    /// `methCompiledBeforeDeprecation`: read from a TASTy file of a version before 3.4 (28.0 to 28.3).
    fn compiled_before_3_4(&self, of: Infixed) -> bool {
        let Some(l) = self.loaded.as_ref() else { return false };
        let file = match of {
            Infixed::Method(s) => l.syms.get(&s).map(|e| e.file),
            Infixed::Class(c) | Infixed::CaseClass(c) => l.classes.get(&c).map(|e| e.file),
            Infixed::StdAlias => None,
            Infixed::Alias(a) => l.aliases.get(&a).map(|e| e.file),
            Infixed::TypeParam | Infixed::Universal(_) | Infixed::NoSymbol => None,
        };
        file.is_some_and(|f| {
            let (major, minor, _) = l.file(f).tasty.version;
            major == 28 && minor < 4
        })
    }
}

#[cfg(test)]
mod tests {
    use super::is_operator_name;

    #[test]
    fn operator_names_as_dottys() {
        for op in ["+", "::", "+:", "a_+", "x_y_<=", "∘", "foo_", "_"] {
            assert!(is_operator_name(op), "{}", op);
        }
        for alpha in ["plus", "max", "a_b", "max=", "x1"] {
            assert!(!is_operator_name(alpha), "{}", alpha);
        }
    }
}
