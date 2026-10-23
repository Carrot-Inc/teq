//! The fix tool's tree as scalameta 4.17.4's parser makes it: a port of `ScalametaParser`
//! (scalameta/parsers/shared/src/main/scala/scala/meta/internal/parsers/ScalametaParser.scala) production by
//! production, in scalameta's names and order, over the scanner's stream (`scanner::scanner`), each node made by the
//! production that consumed its tokens and positioned as scalameta positions it (`pos.rs`). A branch no dialect it
//! reads enables (capture checking's, pure functions') stops the parse (`Stop::NotYet`).

pub(crate) mod chars;
mod defs;
mod names;
mod patterns;
mod pos;
mod stats;
mod stream;
mod terms;
mod types;

pub(crate) use stream::Dialect;

use super::nodes::{flags, Node, Tree, F, K, NONE};
use super::scanner::{self, Scanner};
use super::source::Piece;
use crate::source::Span;

/// Why the parse stopped: a production not ported yet, or scalameta's refusal (its message, at an offset).
#[derive(Debug)]
pub enum Stop {
    NotYet(&'static str),
    Refused(u32, String),
}

pub(crate) type P<T> = Result<T, Stop>;

/// A tree a production made: its scalameta class, its fields in scalameta's order, its value, its position over the
/// scanner's tokens (`[begin, end)`, `Origin.Parsed`'s) and its comments (`begComment`, `endComment`).
struct N {
    kind: K,
    fields: Vec<(F, u32)>,
    value: Option<String>,
    origin: Option<(i32, i32)>,
    beg_comment: Option<(i32, i32)>,
    end_comment: Option<(i32, i32)>,
}

/// The parser's nested contexts (`NestedContext`), each a depth, and the template's owner (`TemplateOwnerContext`).
#[derive(Clone, Copy, Default)]
struct Contexts {
    quoted_splice: u32,
    pattern: u32,
    return_type: u32,
    type_brackets: u32,
    pattern_type: u32,
    extension_sig: u32,
    given_sig: u32,
    template_owner: Owner,
}

/// `TemplateOwner`: what a template belongs to.
#[derive(Clone, Copy, PartialEq, Eq, Default, Debug)]
enum Owner {
    Class,
    Trait,
    #[default]
    Object,
    Enum,
    Given,
}

pub(crate) struct Parser<'a> {
    s: Scanner<'a>,
    /// The tokens the parser reads, in order (`LazyTokenIterator`'s chain), each a `TokenRef` of the scanner's.
    order: Vec<usize>,
    /// The current token's place in `order`.
    i: usize,
    nodes: Vec<N>,
    ctx: Contexts,
    dialect: Dialect,
    /// `TermInfixContext.stack`.
    term_infix: Vec<terms::Unfinished>,
    /// `PatInfixContext.stack`.
    pat_infix: Vec<patterns::UnfinishedPat>,
    /// `TypeInfixContext.stack`.
    type_infix: Vec<types::UnfinishedType>,
}

/// The tree of a document's `pieces` over `text`, as `Parse[Source]` makes it under `dialect`.
pub(crate) fn parse(text: &str, pieces: &[Piece], dialect: Dialect) -> P<Tree> {
    let (s, order) = scanner::scanner(text, pieces, dialect);
    let mut p = Parser { s, order, i: 0, nodes: Vec::new(), ctx: Contexts::default(), dialect, term_infix: Vec::new(), pat_infix: Vec::new(), type_infix: Vec::new() };
    let root = p.parse_rule(|p| p.entrypoint_source())?;
    Ok(p.tree(root))
}

impl<'a> Parser<'a> {
    /// A node of `kind` over `fields`, its value `value`; positioned by the production that made it.
    fn mk(&mut self, kind: K, fields: Vec<(F, u32)>) -> u32 {
        self.mk_full(kind, fields, None)
    }

    fn mk_value(&mut self, kind: K, fields: Vec<(F, u32)>, value: String) -> u32 {
        self.mk_full(kind, fields, Some(value))
    }

    fn mk_full(&mut self, kind: K, fields: Vec<(F, u32)>, value: Option<String>) -> u32 {
        let fields = fields.into_iter().filter(|&(_, c)| c != NONE).collect();
        self.nodes.push(N { kind, fields, value, origin: None, beg_comment: None, end_comment: None });
        (self.nodes.len() - 1) as u32
    }

    fn kind(&self, n: u32) -> K {
        self.nodes[n as usize].kind
    }

    /// The children of `n` in scalameta's order (`Tree.children`).
    fn children(&self, n: u32) -> impl Iterator<Item = u32> + '_ {
        self.nodes[n as usize].fields.iter().map(|&(_, c)| c)
    }

    /// The tree from `root`: its nodes in post-order, a node two parents hold (a clause's `implicit`, each parameter's
    /// too) once under each, a node no parent holds none of it; each span from its tokens' offsets, a template's empty
    /// self (`Template.self`'s answer where none is written) and the flags as the schema has them.
    fn tree(&self, root: u32) -> Tree {
        let mut out = Tree::default();
        let mut made: Vec<u32> = Vec::new();
        let mut stack = vec![(root, false)];
        while let Some((n, done)) = stack.pop() {
            let node = &self.nodes[n as usize];
            if !done {
                stack.push((n, true));
                for &(_, c) in node.fields.iter().rev() {
                    stack.push((c, false));
                }
                continue;
            }
            let kids = made.split_off(made.len() - node.fields.len());
            let mut fields: Vec<(F, u32)> = node.fields.iter().zip(&kids).map(|(&(f, _), &k)| (f, k)).collect();
            let span = self.span_of(n);
            if node.kind == K::Template && !self.has_self(n) {
                let at = node.fields.iter().find(|&&(f, _)| f == F::Body).map_or(span.start, |&(_, b)| self.span_of(b).start);
                let empty = Span::new(at, at);
                let name = push_node(&mut out, K::NameAnonymous, empty, Vec::new(), Some(String::new()));
                fields.push((F::EmptySelf, push_node(&mut out, K::SelfNode, empty, vec![(F::Name, name)], None)));
            }
            let id = push_node(&mut out, node.kind, span, fields, node.value.clone());
            let comments = |c: Option<(i32, i32)>| c.map(|(b, e)| Span::new(self.s.tokens[b as usize].start, self.s.tokens[e as usize - 1].end));
            out.nodes[id as usize].beg_comment = comments(node.beg_comment);
            out.nodes[id as usize].end_comment = comments(node.end_comment);
            made.push(id);
        }
        for k in 0..out.nodes.len() {
            let (first, count) = (out.nodes[k].first as usize, out.nodes[k].count as usize);
            for e in first..first + count {
                let c = out.entries[e].1 as usize;
                out.nodes[c].parent = k as u32;
            }
        }
        set_flags(&mut out);
        out
    }

    /// A template whose body has its self written.
    fn has_self(&self, templ: u32) -> bool {
        self.nodes[templ as usize].fields.iter().any(|&(f, b)| f == F::Body && self.nodes[b as usize].fields.iter().any(|&(g, _)| g == F::SelfDecl))
    }

    /// A node's offsets: its first token's start and its last token's end (`ParsedPartial`), an empty one's at its
    /// token's start.
    fn span_of(&self, n: u32) -> Span {
        let (beg, end) = self.nodes[n as usize].origin.unwrap_or_else(|| panic!("an unpositioned {}", self.nodes[n as usize].kind.name()));
        let toks = &self.s.tokens;
        let start = toks[beg as usize].start;
        if end > beg { Span::new(start, toks[end as usize - 1].end) } else { Span::new(start, start) }
    }
}

fn push_node(out: &mut Tree, kind: K, span: Span, fields: Vec<(F, u32)>, value: Option<String>) -> u32 {
    let first = out.entries.len() as u32;
    let count = fields.len() as u32;
    out.entries.extend(fields);
    let value = value.map_or(NONE, |v| {
        out.values.push(v);
        (out.values.len() - 1) as u32
    });
    out.nodes.push(Node { kind, span, parent: NONE, first, count, value, flags: 0, beg_comment: None, end_comment: None });
    (out.nodes.len() - 1) as u32
}

/// The flags the schema gives (`nodes::flags`): a clause's `using` or `implicit` (a given's conditions `using`), each
/// of its parameters' with it; a parameter's `val` or `var`, a case class's or an enum case's in its first clause that
/// is neither `using` nor `implicit` a `val` (scalac's empty clause stands before an implicit one). A lambda's clause
/// has none, its parameters its `implicit` or `using` (a context function's both) and the one written without its
/// type `UNTYPED`.
fn set_flags(t: &mut Tree) {
    let kind = |t: &Tree, n: u32| t.nodes[n as usize].kind;
    let mods = |t: &Tree, n: u32, f: F| t.children(n).iter().filter(|&&(g, _)| g == f).map(|&(_, m)| t.nodes[m as usize].kind).collect::<Vec<K>>();
    let clause_flags = |t: &Tree, pc: u32| {
        let m = mods(t, pc, F::Mod);
        let group = t.nodes[pc as usize].parent;
        let owner = if group == NONE || kind(t, group) != K::ParamClauseGroup { NONE } else { t.nodes[group as usize].parent };
        let given = owner != NONE && matches!(kind(t, owner), K::DefnGiven | K::DefnGivenAlias | K::DeclGiven | K::DeclGivenAnonymous);
        if m.contains(&K::ModImplicit) {
            flags::IMPLICIT
        } else if m.contains(&K::ModUsing) || given {
            flags::USING
        } else {
            0
        }
    };
    let lambda_of = |t: &Tree, pc: u32| {
        let f = t.nodes[pc as usize].parent;
        (f != NONE && matches!(kind(t, f), K::TermFunction | K::TermContextFunction)).then(|| kind(t, f))
    };
    for n in 0..t.nodes.len() as u32 {
        let f = match kind(t, n) {
            K::ParamClause if lambda_of(t, n).is_some() => 0,
            K::ParamClause => clause_flags(t, n),
            K::Param if t.nodes[n as usize].parent != NONE && lambda_of(t, t.nodes[n as usize].parent).is_some() => {
                let pc = t.nodes[n as usize].parent;
                let mut f = match lambda_of(t, pc) {
                    Some(K::TermContextFunction) => flags::IMPLICIT | flags::USING,
                    _ => clause_flags(t, pc),
                };
                if !t.children(n).iter().any(|&(g, _)| g == F::Decltpe) {
                    f |= flags::UNTYPED;
                }
                f
            }
            K::Param => {
                let pc = t.nodes[n as usize].parent;
                let in_clause = pc != NONE && kind(t, pc) == K::ParamClause;
                let mut f = if in_clause { clause_flags(t, pc) } else { 0 };
                let own = mods(t, n, F::Mods);
                if own.contains(&K::ModVarParam) {
                    f |= flags::VAR;
                } else if own.contains(&K::ModValParam) || in_clause && f & (flags::USING | flags::IMPLICIT) == 0 && first_case_clause(t, pc) {
                    f |= flags::VAL;
                }
                f
            }
            _ => continue,
        };
        t.nodes[n as usize].flags = f;
    }
}

/// `pc` the first clause of a case class's or an enum case's constructor.
fn first_case_clause(t: &Tree, pc: u32) -> bool {
    let ctor = t.nodes[pc as usize].parent;
    if ctor == NONE || t.nodes[ctor as usize].kind != K::CtorPrimary {
        return false;
    }
    if t.children(ctor).iter().find(|&&(f, _)| f == F::ParamClauses).map(|&(_, c)| c) != Some(pc) {
        return false;
    }
    let owner = t.nodes[ctor as usize].parent;
    owner != NONE
        && match t.nodes[owner as usize].kind {
            K::DefnEnumCase => true,
            K::DefnClass => t.children(owner).iter().any(|&(f, m)| f == F::Mods && t.nodes[m as usize].kind == K::ModCase),
            _ => false,
        }
}
