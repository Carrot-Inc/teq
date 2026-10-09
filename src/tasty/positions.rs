//! The positions of a TASTy file's trees: the `Positions`
//! section read as scalac 3.8.4's `PositionUnpickler` reads it, and the span `TreeUnpickler`
//! gives each tree, which is the section's entry where the tree has one and otherwise what
//! `Positioned.envelope` reconstructs: the union of the children of the tree's source, an
//! `Inlined` the span of its call, a child without a span given a point at its neighbours'
//! boundary. The section holds an entry only for a tree whose span the envelope does not give
//! (`PositionPickler` 119 to 121) and for every definition.
//!
//! Spans count UTF-16 units of the source, as scalac's `SourceFile` does. A tree's source is the
//! path of the innermost `SOURCE` switch whose tree holds it: the pickler writes one at the
//! address of every tree whose source is not its parent's on some path to it, so that the
//! containment of addresses decides it, but for a shared reference, which reads its target in
//! the occurrence's context: the switch at the target where there is one, the occurrence's
//! source where not (which is then the target's, on every path).

use super::tags::*;
use super::tree::Addr;
use super::{NameRef, Reader, TastyFile};
use crate::intern::FxMap;

/// A span in UTF-16 units; `point` is absent for a synthetic span.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Pos {
    pub start: u32,
    pub end: u32,
    pub point: Option<u32>,
}

/// The `Positions` section as read: the primary source's line sizes, the explicit spans by
/// address (the last of an address kept, as scalac's map keeps it) and the source switches.
pub struct Section {
    /// The size of each line of the primary source in UTF-16 units, its `\n` left out.
    pub lines: Vec<u32>,
    entries: Vec<(Addr, Pos)>,
    /// Per switch the address of the tree it starts, the end of that tree and the path.
    switches: Vec<(Addr, Addr, NameRef)>,
}

const SOURCE: i64 = 4;

impl Section {
    /// The section of `file`, `None` when it has none. A section cut short gives what was read
    /// before the cut.
    pub fn read(file: &TastyFile) -> Option<Section> {
        if file.positions.is_empty() {
            return None;
        }
        let bytes = &file.bytes[file.positions.clone()];
        let mut r = Reader::new(bytes);
        let count = r.nat() as usize;
        let mut lines = Vec::with_capacity(count.min(bytes.len()));
        for _ in 0..count {
            if r.at_end() {
                break;
            }
            // Earlier compilers wrote -1 as a Nat for a line of unknown size.
            let n = r.long_nat();
            lines.push(if n == 0xFFFF_FFFF { u32::MAX } else { n as u32 });
        }
        let mut entries: Vec<(Addr, Pos)> = Vec::new();
        let mut sources: Vec<(Addr, NameRef)> = Vec::new();
        let (mut addr, mut start, mut end) = (0i64, 0i64, 0i64);
        while !r.at_end() {
            let header = r.long_int();
            if header == SOURCE {
                let name = r.long_int();
                if addr >= 0 && name >= 0 {
                    sources.push((addr as Addr, name as NameRef));
                }
                continue;
            }
            addr += header >> 3;
            if header & 4 != 0 {
                start += r.long_int();
            }
            if header & 2 != 0 {
                end += r.long_int();
            }
            let point = (header & 1 != 0).then(|| start + r.long_int());
            if addr < 0 || start < 0 || end < start {
                continue;
            }
            let point = point.filter(|&p| p >= 0).map(|p| p as u32);
            entries.push((addr as Addr, Pos { start: start as u32, end: end as u32, point }));
        }
        // The last entry of an address wins; the sort is stable.
        entries.sort_by_key(|&(a, _)| a);
        let mut kept: Vec<(Addr, Pos)> = Vec::with_capacity(entries.len());
        for e in entries {
            match kept.last_mut() {
                Some(last) if last.0 == e.0 => *last = e,
                _ => kept.push(e),
            }
        }
        let trees = &file.bytes[file.asts.clone()];
        let mut switches: Vec<(Addr, Addr, NameRef)> =
            sources.into_iter().filter(|&(a, _)| (a as usize) < trees.len()).map(|(a, n)| (a, tree_end(trees, a), n)).collect();
        switches.sort_by_key(|&(a, e, _)| (a, std::cmp::Reverse(e)));
        switches.dedup_by_key(|s| s.0);
        Some(Section { lines, entries: kept, switches })
    }

    pub fn entry(&self, addr: Addr) -> Option<Pos> {
        self.entries.binary_search_by_key(&addr, |&(a, _)| a).ok().map(|i| self.entries[i].1)
    }

    /// The path of the source of the tree at `addr`: the innermost switch whose tree holds it.
    pub fn source_at(&self, addr: Addr) -> Option<NameRef> {
        let upto = self.switches.partition_point(|&(a, _, _)| a <= addr);
        self.switches[..upto].iter().rev().find(|&&(a, e, _)| a <= addr && addr < e).map(|&(_, _, n)| n)
    }

    /// The path of a switch at `addr` itself.
    pub fn switch_at(&self, addr: Addr) -> Option<NameRef> {
        self.switches.binary_search_by_key(&addr, |&(a, _, _)| a).ok().map(|i| self.switches[i].2)
    }

    /// The primary source: the switch at the root.
    pub fn primary(&self) -> Option<NameRef> {
        self.switches.iter().find(|&&(a, _, _)| a == 0).map(|&(_, _, n)| n)
    }

    /// The 1-based line and column (in UTF-16 units, from 1) of an offset of the primary source,
    /// from the line table; `None` past its end or where a line's size is unknown.
    pub fn line_col(&self, offset: u32) -> Option<(u32, u32)> {
        let mut line_start = 0u64;
        for (i, &size) in self.lines.iter().enumerate() {
            if size == u32::MAX {
                return None;
            }
            let line_end = line_start + size as u64;
            if (offset as u64) <= line_end {
                return Some((i as u32 + 1, (offset as u64 - line_start) as u32 + 1));
            }
            line_start = line_end + 1;
        }
        None
    }
}

/// The address past the tree at `addr`.
fn tree_end(trees: &[u8], addr: Addr) -> Addr {
    let mut r = Reader::new(trees);
    r.pos = addr as usize;
    skip(&mut r);
    r.pos.min(trees.len()) as Addr
}

fn skip(r: &mut Reader) {
    let tag = r.byte();
    if tag >= PACKAGE {
        r.pos = r.end();
    } else if tag >= IDENT {
        r.nat();
        skip(r);
    } else if tag >= THIS {
        skip(r);
    } else if tag >= SHAREDTERM {
        r.nat();
    }
}

/// What a tree's reconstruction gave it.
#[derive(Clone, Copy, Debug)]
pub struct Placed {
    pub tag: u8,
    /// A shared reference's target, whose tree the occurrence reads.
    pub target: Option<Addr>,
    pub pos: Option<Pos>,
    pub source: Option<NameRef>,
}

/// The span of every tree of a file, keyed by address, as `TreeUnpickler` gives it.
#[derive(Default)]
pub struct Spans {
    map: FxMap<Addr, Placed>,
}

impl Spans {
    /// Every tree of the file, from its top-level statements.
    pub fn of(file: &TastyFile, section: &Section) -> Spans {
        let trees = &file.bytes[file.asts.clone()];
        let mut w = Walk { trees, section, nodes: Vec::new(), children: Vec::new(), scratch: Vec::new(), depth: 0, shared: FxMap::default(), copies: FxMap::default(), open: Vec::new() };
        let mut r = Reader::new(trees);
        let mut roots = Vec::new();
        while !r.at_end() {
            let before = r.pos;
            roots.push(w.tree(&mut r, Mode::Stat));
            if r.pos <= before {
                break;
            }
        }
        let mut map: FxMap<Addr, Placed> = FxMap::default();
        for n in &w.nodes {
            if n.copy {
                continue;
            }
            map.entry(n.addr).or_insert(Placed { tag: n.tag, target: n.target, pos: n.span, source: n.source });
        }
        // A shared tree whose target is no tree of its own (an annotation's argument in a type,
        // `@Repeated` of a repeated parameter's): `TreeUnpickler` first reads it as a tree at the
        // occurrence, every node of it at the occurrence's span where the target has none.
        let occurrences: Vec<(Addr, Option<Pos>, Option<NameRef>)> = w.nodes.iter().filter(|n| !n.copy).filter_map(|n| n.target.map(|t| (t, n.span, n.source))).collect();
        for (t, span, source) in occurrences {
            if map.contains_key(&t) {
                continue;
            }
            for c in &w.nodes[w.copies.get(&t).cloned().unwrap_or(0..0)] {
                map.entry(c.addr).or_insert(Placed { tag: c.tag, target: c.target, pos: c.span.or(span), source: c.source.or(source) });
            }
        }
        Spans { map }
    }

    pub fn get(&self, addr: Addr) -> Option<Placed> {
        self.map.get(&addr).copied()
    }

    /// The trees by address, in address order.
    pub fn sorted(&self) -> Vec<(Addr, Placed)> {
        let mut v: Vec<(Addr, Placed)> = self.map.iter().map(|(&a, &p)| (a, p)).collect();
        v.sort_by_key(|&(a, _)| a);
        v
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Mode {
    /// A statement: a definition, an import, or a term.
    Stat,
    Term,
    /// A type tree: a tree tag, or a type read as a `TypeTree`.
    Tpt,
    Case,
}

struct Node {
    addr: Addr,
    tag: u8,
    source: Option<NameRef>,
    span: Option<Pos>,
    children: (u32, u32),
    /// An `INLINED` with its call as the first child: its envelope is the call's span.
    inlined: bool,
    /// The children that take part in the envelope: a definition's right-hand side and a
    /// template's body are read lazily by scalac, after the definition's envelope.
    eager: u32,
    /// A tree read again through a shared reference, which `TreeUnpickler` copies: its span
    /// counts for the occurrence, the target's own tree keeps the target's.
    copy: bool,
    target: Option<Addr>,
}

const MAX_DEPTH: u32 = 1024;
const MAX: u32 = u32::MAX;

struct Walk<'a> {
    trees: &'a [u8],
    section: &'a Section,
    nodes: Vec<Node>,
    children: Vec<u32>,
    scratch: Vec<u32>,
    depth: u32,
    /// The span a shared tree's reading gives at its target, which the occurrence keeps where
    /// it has no entry of its own.
    shared: FxMap<(Addr, bool), Option<Pos>>,
    /// The nodes of each shared tree's reading at its target, by the target.
    copies: FxMap<Addr, std::ops::Range<usize>>,
    open: Vec<Addr>,
}

impl<'a> Walk<'a> {
    fn leaf(&mut self, addr: Addr, tag: u8) -> u32 {
        let span = self.section.entry(addr);
        let source = self.section.source_at(addr);
        self.nodes.push(Node { addr, tag, source, span, children: (0, 0), inlined: false, eager: 0, copy: false, target: None });
        (self.nodes.len() - 1) as u32
    }

    /// Reads the tree at the cursor in `mode` and returns its node, its children read first.
    fn tree(&mut self, r: &mut Reader, mode: Mode) -> u32 {
        let addr = r.pos as Addr;
        if self.depth >= MAX_DEPTH {
            skip(r);
            return self.leaf(addr, 0);
        }
        self.depth += 1;
        let n = self.tree_now(r, mode, addr);
        self.depth -= 1;
        n
    }

    fn tree_now(&mut self, r: &mut Reader, mode: Mode, addr: Addr) -> u32 {
        let tag = r.peek();
        let mark = self.scratch.len();
        // Children are pushed in the order of scalac's tree's fields; `eager` counts those its
        // constructor sees.
        let mut eager: Option<u32> = None;
        let mut inlined = false;
        match tag {
            SHAREDTERM => {
                r.byte();
                let target = r.nat();
                let span = self.shared_span(target, mode);
                // A shared case is read at its target and keeps the target's span (`readCases`).
                let own = if mode == Mode::Case { None } else { self.section.entry(addr) };
                // Read at the target in the occurrence's context, which a switch at the target
                // changes (`sourceChangeContext`): the pickler writes one there whenever a path to
                // the tree comes from another source.
                let source = self.section.switch_at(target).or_else(|| self.section.source_at(addr));
                self.nodes.push(Node { addr, tag, source, span: own.or(span), children: (0, 0), inlined: false, eager: 0, copy: false, target: Some(target) });
                return (self.nodes.len() - 1) as u32;
            }
            VALDEF | DEFDEF | TYPEPARAM | PARAM => {
                r.byte();
                let end = r.end();
                r.nat();
                if tag == DEFDEF {
                    while r.pos < end && matches!(r.peek(), TYPEPARAM | PARAM | EMPTYCLAUSE | SPLITCLAUSE) {
                        if matches!(r.peek(), EMPTYCLAUSE | SPLITCLAUSE) {
                            r.byte();
                        } else {
                            let p = self.tree(r, Mode::Stat);
                            self.scratch.push(p);
                        }
                    }
                }
                if r.pos < end && !is_modifier(r.peek()) {
                    let t = self.tree(r, Mode::Tpt);
                    self.scratch.push(t);
                }
                eager = Some((self.scratch.len() - mark) as u32);
                if matches!(tag, VALDEF | DEFDEF) && r.pos < end && !is_modifier(r.peek()) {
                    let rhs = self.tree(r, Mode::Term);
                    self.scratch.push(rhs);
                }
                self.annotations(r, end);
                r.pos = end;
            }
            TYPEDEF => {
                r.byte();
                let end = r.end();
                r.nat();
                if r.pos < end {
                    let rhs = if r.peek() == TEMPLATE { self.template(r) } else { self.tree(r, Mode::Tpt) };
                    self.scratch.push(rhs);
                }
                self.annotations(r, end);
                r.pos = end;
            }
            TEMPLATE => return self.template(r),
            PACKAGE => {
                r.byte();
                let end = r.end();
                let pid = self.tree(r, Mode::Term);
                self.scratch.push(pid);
                while r.pos < end {
                    let before = r.pos;
                    let s = self.tree(r, Mode::Stat);
                    self.scratch.push(s);
                    if r.pos <= before {
                        break;
                    }
                }
                r.pos = end;
            }
            IMPORT | EXPORT => {
                r.byte();
                let end = r.end();
                let q = self.tree(r, Mode::Term);
                self.scratch.push(q);
                while r.pos < end {
                    let at = r.pos as Addr;
                    match r.byte() {
                        IMPORTED | RENAMED => {
                            r.nat();
                            let s = self.leaf(at, IMPORTED);
                            self.scratch.push(s);
                        }
                        BOUNDED => {
                            let t = self.tree(r, Mode::Tpt);
                            self.scratch.push(t);
                        }
                        _ => break,
                    }
                }
                r.pos = end;
            }
            _ if mode == Mode::Stat => return self.tree_now(r, Mode::Term, addr),
            // A type tree tag, or a type that `readTpt` reads as a `TypeTree` leaf.
            _ if mode == Mode::Tpt && !is_type_tree(tag) && !matches!(tag, BLOCK | HOLE) => {
                skip(r);
                return self.leaf(addr, tag);
            }
            BLOCK => {
                r.byte();
                let end = r.end();
                // Scalac's `Block(stats, expr)` holds the statements first.
                let mut sub = Reader { bytes: r.bytes, pos: r.pos };
                skip(&mut sub);
                let expr_at = r.pos;
                r.pos = sub.pos;
                while r.pos < end {
                    let before = r.pos;
                    let s = self.tree(r, Mode::Stat);
                    self.scratch.push(s);
                    if r.pos <= before {
                        break;
                    }
                }
                let mut er = Reader { bytes: r.bytes, pos: expr_at };
                let e = self.tree(&mut er, if mode == Mode::Tpt { Mode::Tpt } else { Mode::Term });
                self.scratch.push(e);
                r.pos = end;
            }
            INLINED => {
                r.byte();
                let end = r.end();
                let expansion_at = r.pos;
                skip(r);
                let mut call = None;
                if r.pos < end && !matches!(unshared_tag(r), VALDEF | DEFDEF) {
                    call = Some(self.tree(r, Mode::Term));
                }
                let mut bindings = Vec::new();
                while r.pos < end {
                    let before = r.pos;
                    bindings.push(self.tree(r, Mode::Stat));
                    if r.pos <= before {
                        break;
                    }
                }
                let mut er = Reader { bytes: r.bytes, pos: expansion_at };
                let expansion = self.tree(&mut er, Mode::Term);
                inlined = call.is_some();
                self.scratch.extend(call);
                self.scratch.extend(bindings);
                self.scratch.push(expansion);
                if call.is_none() {
                    // `Inlined(EmptyTree, ..)`: the envelope is the empty call's, none.
                    eager = Some(0);
                }
                r.pos = end;
            }
            CASEDEF => {
                r.byte();
                let end = r.end();
                let pat = self.tree(r, Mode::Term);
                let rhs = self.tree(r, Mode::Term);
                let guard = (r.pos < end).then(|| self.tree(r, Mode::Term));
                self.scratch.push(pat);
                self.scratch.extend(guard);
                self.scratch.push(rhs);
                r.pos = end;
            }
            REPEATED => {
                r.byte();
                let end = r.end();
                let elem = self.tree(r, Mode::Tpt);
                while r.pos < end {
                    let before = r.pos;
                    let e = self.tree(r, Mode::Term);
                    self.scratch.push(e);
                    if r.pos <= before {
                        break;
                    }
                }
                self.scratch.push(elem);
                r.pos = end;
            }
            QUOTEPATTERN => {
                r.byte();
                let end = r.end();
                let body = self.tree(r, Mode::Term);
                let quotes = self.tree(r, Mode::Term);
                skip(r);
                while r.pos < end {
                    let before = r.pos;
                    let s = self.tree(r, Mode::Stat);
                    self.scratch.push(s);
                    if r.pos <= before {
                        break;
                    }
                }
                self.scratch.push(body);
                self.scratch.push(quotes);
                r.pos = end;
            }
            RETURN => {
                r.byte();
                let end = r.end();
                r.nat();
                if r.pos < end {
                    let e = self.tree(r, Mode::Term);
                    self.scratch.push(e);
                }
                r.pos = end;
            }
            SELECTOUTER => {
                r.byte();
                let end = r.end();
                r.nat();
                let q = self.tree(r, Mode::Term);
                self.scratch.push(q);
                r.pos = end;
            }
            SELECTIN => {
                r.byte();
                let end = r.end();
                r.nat();
                let q = self.tree(r, Mode::Term);
                self.scratch.push(q);
                r.pos = end;
            }
            APPLYSIGPOLY => {
                r.byte();
                let end = r.end();
                let f = self.tree(r, Mode::Term);
                self.scratch.push(f);
                skip(r);
                self.until(r, end, Mode::Term);
            }
            TYPEAPPLY => {
                r.byte();
                let end = r.end();
                let f = self.tree(r, Mode::Term);
                self.scratch.push(f);
                self.until(r, end, Mode::Tpt);
            }
            TYPED => {
                r.byte();
                let end = r.end();
                let e = self.tree(r, Mode::Term);
                let t = self.tree(r, Mode::Tpt);
                self.scratch.push(e);
                self.scratch.push(t);
                r.pos = end;
            }
            SUPER => {
                r.byte();
                let end = r.end();
                let q = self.tree(r, Mode::Term);
                self.scratch.push(q);
                if r.pos < end {
                    let m = self.tree(r, Mode::Tpt);
                    self.scratch.push(m);
                }
                r.pos = end;
            }
            LAMBDA => {
                r.byte();
                let end = r.end();
                let m = self.tree(r, Mode::Term);
                self.scratch.push(m);
                if r.pos < end {
                    let t = self.tree(r, Mode::Tpt);
                    self.scratch.push(t);
                }
                r.pos = end;
            }
            IF | WHILE | ASSIGN | APPLY | ALTERNATIVE => {
                r.byte();
                let end = r.end();
                if tag == IF && r.peek() == INLINE {
                    r.byte();
                }
                self.until(r, end, Mode::Term);
            }
            MATCH | TRY => {
                r.byte();
                let end = r.end();
                match r.peek() {
                    IMPLICIT => {
                        r.byte();
                    }
                    INLINE | SUBMATCH => {
                        r.byte();
                        let s = self.tree(r, Mode::Term);
                        self.scratch.push(s);
                    }
                    _ => {
                        let s = self.tree(r, Mode::Term);
                        self.scratch.push(s);
                    }
                }
                while r.pos < end && unshared_tag(r) == CASEDEF {
                    let c = self.tree(r, Mode::Case);
                    self.scratch.push(c);
                }
                if tag == TRY && r.pos < end {
                    let f = self.tree(r, Mode::Term);
                    self.scratch.push(f);
                }
                r.pos = end;
            }
            BIND => {
                r.byte();
                let end = r.end();
                r.nat();
                skip(r);
                if r.pos < end && !is_modifier(r.peek()) {
                    let p = self.tree(r, Mode::Term);
                    self.scratch.push(p);
                }
                r.pos = end;
            }
            UNAPPLY => {
                r.byte();
                let end = r.end();
                let f = self.tree(r, Mode::Term);
                self.scratch.push(f);
                while r.pos < end && r.peek() == IMPLICITARG {
                    r.byte();
                    let a = self.tree(r, Mode::Term);
                    self.scratch.push(a);
                }
                skip(r);
                self.until(r, end, Mode::Term);
            }
            QUOTE | SPLICE => {
                r.byte();
                let end = r.end();
                let b = self.tree(r, Mode::Term);
                self.scratch.push(b);
                r.pos = end;
            }
            SPLICEPATTERN => {
                r.byte();
                let end = r.end();
                let p = self.tree(r, Mode::Term);
                self.scratch.push(p);
                skip(r);
                self.until(r, end, Mode::Term);
            }
            HOLE => {
                r.byte();
                let end = r.end();
                r.nat();
                skip(r);
                self.until(r, end, Mode::Term);
            }
            REFINEDTPT => {
                r.byte();
                let end = r.end();
                let p = self.tree(r, Mode::Tpt);
                self.scratch.push(p);
                self.until(r, end, Mode::Stat);
            }
            APPLIEDTPT | TYPEBOUNDSTPT => {
                r.byte();
                let end = r.end();
                self.until(r, end, Mode::Tpt);
            }
            ANNOTATEDTPT => {
                r.byte();
                let end = r.end();
                let t = self.tree(r, Mode::Tpt);
                self.scratch.push(t);
                if r.pos < end {
                    let a = self.tree(r, Mode::Term);
                    self.scratch.push(a);
                }
                r.pos = end;
            }
            LAMBDATPT => {
                r.byte();
                let end = r.end();
                while r.pos < end && r.peek() == TYPEPARAM {
                    let p = self.tree(r, Mode::Stat);
                    self.scratch.push(p);
                }
                if r.pos < end {
                    let b = self.tree(r, Mode::Tpt);
                    self.scratch.push(b);
                }
                r.pos = end;
            }
            MATCHTPT => {
                r.byte();
                let end = r.end();
                while r.pos < end && unshared_tag(r) != CASEDEF {
                    let t = self.tree(r, Mode::Tpt);
                    self.scratch.push(t);
                }
                while r.pos < end && unshared_tag(r) == CASEDEF {
                    let c = self.tree(r, Mode::Case);
                    self.scratch.push(c);
                }
                r.pos = end;
            }
            // `EXPLICITtpt` marks the type tree after it where a term could stand (a quote
            // pattern's `'[T]`, a hole's type argument): scalac reads that tree, no tree of the
            // marker's own.
            EXPLICITTPT => {
                r.byte();
                return self.tree(r, Mode::Tpt);
            }
            // `This(qual)`, whose qualifier is an identifier with the type tree's span.
            QUALTHIS | NEW | BYNAMETPT => {
                r.byte();
                let t = self.tree(r, Mode::Tpt);
                self.scratch.push(t);
            }
            // `Throw(expr)` is an application of `throw` to the expression.
            THROW | SINGLETONTPT => {
                r.byte();
                let t = self.tree(r, Mode::Term);
                self.scratch.push(t);
            }
            SELECT | SELECTTPT | NAMEDARG => {
                r.byte();
                r.nat();
                let q = self.tree(r, Mode::Term);
                self.scratch.push(q);
            }
            // Identifiers, paths, constants and `ELIDED`: leaves.
            _ => {
                skip(r);
                return self.leaf(addr, tag);
            }
        }
        self.finish(addr, tag, mark, eager, inlined)
    }

    fn until(&mut self, r: &mut Reader, end: usize, mode: Mode) {
        while r.pos < end {
            let before = r.pos;
            let t = self.tree(r, mode);
            self.scratch.push(t);
            if r.pos <= before {
                break;
            }
        }
        r.pos = end;
    }

    /// The annotations among a definition's modifiers: their trees have positions, and are no
    /// children of the definition's tree.
    fn annotations(&mut self, r: &mut Reader, end: usize) {
        while r.pos < end {
            match r.byte() {
                ANNOTATION => {
                    let a_end = r.end();
                    skip(r);
                    if r.pos < a_end {
                        let mark = self.scratch.len();
                        let t = self.tree(r, Mode::Term);
                        self.scratch.truncate(mark);
                        let _ = t;
                    }
                    r.pos = a_end;
                }
                PRIVATEQUALIFIED | PROTECTEDQUALIFIED => skip(r),
                t if is_modifier(t) => {}
                _ => break,
            }
        }
    }

    fn template(&mut self, r: &mut Reader) -> u32 {
        let addr = r.pos as Addr;
        let mark = self.scratch.len();
        r.byte();
        let end = r.end();
        let mut params = Vec::new();
        while r.pos < end && matches!(r.peek(), TYPEPARAM | PARAM) {
            params.push(self.tree(r, Mode::Stat));
        }
        let mut parents = Vec::new();
        while r.pos < end && !matches!(r.peek(), SELFDEF | DEFDEF | SPLITCLAUSE) {
            let mode = if matches!(unshared_tag(r), APPLY | TYPEAPPLY | BLOCK) { Mode::Term } else { Mode::Tpt };
            let before = r.pos;
            parents.push(self.tree(r, mode));
            if r.pos <= before {
                break;
            }
        }
        // `ValDef(self, tpt)`, made without a span of its own: its tpt's.
        let mut self_def = None;
        if r.peek() == SELFDEF {
            let at = r.pos as Addr;
            r.byte();
            r.nat();
            let mark = self.scratch.len();
            let tpt = self.tree(r, Mode::Tpt);
            self.scratch.push(tpt);
            let n = self.finish(at, SELFDEF, mark, None, false);
            if self.section.entry(at).is_some() {
                self.nodes[n as usize].span = self.nodes[tpt as usize].span;
            }
            self_def = Some(n);
        }
        if r.peek() == SPLITCLAUSE {
            r.byte();
        }
        let mut constr = None;
        if r.pos < end && r.peek() == DEFDEF {
            constr = Some(self.tree(r, Mode::Stat));
        }
        let mut stats = Vec::new();
        while r.pos < end {
            let before = r.pos;
            stats.push(self.tree(r, Mode::Stat));
            if r.pos <= before {
                break;
            }
        }
        r.pos = end;
        // `Template(constr, parents, self, body)`, the body (the parameters and the statements)
        // read lazily.
        self.scratch.extend(constr);
        self.scratch.extend(parents);
        self.scratch.extend(self_def);
        let eager = (self.scratch.len() - mark) as u32;
        self.scratch.extend(params);
        self.scratch.extend(stats);
        self.finish(addr, TEMPLATE, mark, Some(eager), false)
    }

    /// Makes the node of the tree at `addr` from the children pushed since `mark`, its span
    /// the entry or else the envelope, as scalac's constructor and `setSpan` give it.
    fn finish(&mut self, addr: Addr, tag: u8, mark: usize, eager: Option<u32>, inlined: bool) -> u32 {
        let first = self.children.len() as u32;
        self.children.extend(self.scratch.drain(mark..));
        let count = self.children.len() as u32 - first;
        let source = self.section.source_at(addr);
        self.nodes.push(Node { addr, tag, source, span: None, children: (first, count), inlined, eager: eager.unwrap_or(count), copy: false, target: None });
        let n = (self.nodes.len() - 1) as u32;
        let envelope = self.envelope(n, None);
        self.nodes[n as usize].span = self.section.entry(addr).or(envelope);
        n
    }

    /// `Positioned.envelope`: the union of the spans of the children of the node's source from
    /// `start`, a child without a span given a point at the end of what comes before it (or at
    /// the start of what comes after), synthetic; an `Inlined`'s is its call's.
    fn envelope(&mut self, n: u32, start: Option<(u32, u32)>) -> Option<Pos> {
        let node = &self.nodes[n as usize];
        let (first, _) = node.children;
        let eager = node.eager;
        if node.inlined {
            let call = self.children[first as usize];
            return self.nodes[call as usize].span;
        }
        if node.tag == INLINED {
            return None;
        }
        let src = node.source;
        let kids: Vec<u32> = self.children[first as usize..(first + eager) as usize].to_vec();
        let pass = |w: &mut Walk, start: Option<(u32, u32)>| -> Option<(u32, u32)> {
            let mut span = start;
            for &k in &kids {
                if w.nodes[k as usize].source != src {
                    continue;
                }
                match (w.nodes[k as usize].span, span) {
                    (Some(p), _) => span = Some(union(span, (p.start, p.end))),
                    (None, Some(s)) => {
                        if s.1 != MAX {
                            let filled = w.envelope(k, Some((s.1, s.1)));
                            w.nodes[k as usize].span = filled;
                        }
                    }
                    (None, None) => span = Some((MAX, MAX)),
                }
            }
            span
        };
        let first_pass = pass(self, start);
        let span = match first_pass {
            None => None,
            Some(s) if s.1 != MAX => Some(s),
            Some(s) if s.0 == MAX => None,
            Some(s) => pass(self, Some((s.0, s.0))),
        };
        span.map(|(s, e)| Pos { start: s, end: e, point: None })
    }

    /// The span a reading of the tree at `target` gives, as a shared reference reads it.
    fn shared_span(&mut self, target: Addr, mode: Mode) -> Option<Pos> {
        let key = (target, mode == Mode::Case);
        if let Some(&s) = self.shared.get(&key) {
            return s;
        }
        if self.open.contains(&target) || target as usize >= self.trees.len() {
            return None;
        }
        self.open.push(target);
        let mut r = Reader::new(self.trees);
        r.pos = target as usize;
        let marks = (self.nodes.len(), self.scratch.len());
        let n = self.tree(&mut r, if mode == Mode::Stat { Mode::Term } else { mode });
        let span = self.nodes[n as usize].span;
        for node in &mut self.nodes[marks.0..] {
            node.copy = true;
        }
        self.copies.entry(target).or_insert(marks.0..self.nodes.len());
        self.scratch.truncate(marks.1);
        self.open.pop();
        self.shared.insert(key, span);
        span
    }
}

fn union(span: Option<(u32, u32)>, (s, e): (u32, u32)) -> (u32, u32) {
    match span {
        None => (s, e),
        Some((a, b)) => (a.min(s), b.max(e)),
    }
}

fn unshared_tag(r: &Reader) -> u8 {
    let mut c = r.clone();
    if c.peek() == SHAREDTERM {
        c.byte();
        let addr = c.nat() as usize;
        c.pos = addr;
    }
    c.peek()
}

/// The tags `TreeUnpickler.isTypeTreeTag` reads as type trees.
fn is_type_tree(tag: u8) -> bool {
    matches!(tag, IDENTTPT | SELECTTPT | SINGLETONTPT | REFINEDTPT | APPLIEDTPT | LAMBDATPT | TYPEBOUNDSTPT | ANNOTATEDTPT | BYNAMETPT | MATCHTPT | EXPLICITTPT | BIND)
}

/// `teq tasty --positions`: every tree by address with its tag, its span (`none` without one),
/// its point where the span is not synthetic, and its source's path.
pub fn listing(name: &str, file: &TastyFile) -> String {
    let mut out = format!("// {}\n", name);
    let Some(section) = Section::read(file) else {
        out.push_str("no Positions section\n");
        return out;
    };
    let spans = Spans::of(file, &section);
    for (addr, p) in spans.sorted() {
        let span = match p.pos {
            Some(pos) => match pos.point {
                Some(pt) => format!("{} .. {} point {}", pos.start, pos.end, pt),
                None => format!("{} .. {}", pos.start, pos.end),
            },
            None => "none".to_string(),
        };
        let source = p.source.map_or("-".to_string(), |n| file.name(n));
        let tag = match p.target.and_then(|t| spans.get(t)) {
            Some(t) => format!("{}({})", super::dump::tag_name(p.tag), super::dump::tag_name(t.tag)),
            None => super::dump::tag_name(p.tag).to_string(),
        };
        out.push_str(&format!("{} {} {} {}\n", addr, tag, span, source));
    }
    out
}
