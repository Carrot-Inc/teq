//! The builtins of `teq.fix.Native` (`std/fix/native.scala`): what a rule of `teq fix` reads of its document and
//! hands back, through the rule thread's host (`fix::host`). A node or token is its document's generation and its
//! index; one of a document the rules no longer read throws `IllegalStateException`.

use super::builtins::{reg, Table};
use super::value::{ArrayCell, Value};
use super::{Interp, R};
use crate::fix::config::Value as Conf;
use crate::fix::host::{with_host, DocView, Edit, Host, ImportOp, Lint, Op, Severity};
use crate::fix::service::{Answer, Ask};
use crate::fix::nodes::{FIELD_NAMES, KIND_NAMES, NONE};
use crate::source::Span;
use std::rc::Rc;

// A member of the object `Native` is called with the object first: its arguments start after it.
fn int(a: &[Value], i: usize) -> i32 {
    a.get(i + 1).and_then(Value::as_i32).unwrap_or(-1)
}

fn text(a: &[Value], i: usize) -> String {
    a.get(i + 1).and_then(Value::as_str).unwrap_or("").to_string()
}

/// The `Int`s of the `Array` argument `i`.
fn int_items(a: &[Value], i: usize) -> Vec<i32> {
    match a.get(i + 1) {
        Some(Value::Array(cell)) => cell.borrow().iter().filter_map(Value::as_i32).collect(),
        _ => Vec::new(),
    }
}

/// Which of `n` codes `codes` names, ignoring a code outside them.
fn wanted_codes(codes: Vec<i32>, n: usize) -> Vec<bool> {
    let mut want = vec![false; n];
    for c in codes {
        if let Some(w) = usize::try_from(c).ok().and_then(|c| want.get_mut(c)) {
            *w = true;
        }
    }
    want
}

fn ints(v: impl IntoIterator<Item = i32>) -> Value {
    Value::Array(ArrayCell::new(v.into_iter().map(Value::Int).collect()))
}

fn strings(v: impl IntoIterator<Item = String>) -> Value {
    Value::Array(ArrayCell::new(v.into_iter().map(Value::string).collect()))
}

/// `f` over the document of generation `gen`, or the exception a rule meets for a stale handle or outside a run.
fn on_doc(it: &mut Interp<'_, '_>, gen: i32, f: impl FnOnce(&DocView) -> Result<Value, String>) -> R {
    let answer = with_host(|h| match &h.doc {
        Some(d) if d.gen as i32 == gen => f(d),
        Some(_) => Err("a tree or token of another document than the one the rule reads".to_string()),
        None => Err("no document is read".to_string()),
    });
    match answer {
        Some(Ok(v)) => Ok(v),
        Some(Err(msg)) => it.throw_named("IllegalStateException", &msg),
        None => it.throw_named("IllegalStateException", "teq.fix runs under teq fix alone"),
    }
}

fn on_host(it: &mut Interp<'_, '_>, f: impl FnOnce(&mut Host) -> Result<Value, String>) -> R {
    match with_host(f) {
        Some(Ok(v)) => Ok(v),
        Some(Err(msg)) => it.throw_named("IllegalStateException", &msg),
        None => it.throw_named("IllegalStateException", "teq.fix runs under teq fix alone"),
    }
}

/// The answer of the round's SemanticDB service (`fix::service`) to the question `ask` makes of the document `gen`, read
/// by `read`, or the exception a rule meets where there is none: a stale handle, a run with no semantic rule, the
/// service's failure or its refusal.
fn query(it: &mut Interp<'_, '_>, gen: i32, ask: impl FnOnce(&DocView) -> Result<Ask, String>, read: impl FnOnce(Answer) -> Result<Value, String>) -> R {
    let asked = with_host(|h| {
        let d = match &h.doc {
            Some(d) if d.gen as i32 == gen => d,
            Some(_) => return Err("a tree or token of another document than the one the rule reads".to_string()),
            None => return Err("no document is read".to_string()),
        };
        let ask = ask(d)?;
        let link = h.sem.clone().ok_or("no semantic rule is configured, so the run answers no SemanticDB question")?;
        let a = h.asker.ok_or("no document is read")?;
        h.queries += 1;
        Ok((link, (a.job, a.round, a.gen), ask))
    });
    let answer = match asked {
        None => return it.throw_named("IllegalStateException", "teq.fix runs under teq fix alone"),
        Some(Err(msg)) => return it.throw_named("IllegalStateException", &msg),
        Some(Ok((link, key, ask))) => crate::fix::service::ask(&link, key, ask),
    };
    match answer.and_then(read) {
        Ok(v) => Ok(v),
        Err(msg) => it.throw_named("IllegalStateException", &msg),
    }
}

/// The refusal of an answer to another question than `what`'s.
fn other(what: &str) -> String {
    format!("the SemanticDB service answered another question than {}'s", what)
}

fn bytes(b: Vec<u8>) -> Value {
    Value::Array(ArrayCell::new(b.into_iter().map(|x| Value::Byte(x as i8)).collect()))
}

fn node(d: &DocView, n: i32) -> Result<&crate::fix::nodes::Node, String> {
    d.tree.nodes.get(n as usize).filter(|_| n >= 0).ok_or_else(|| format!("no node {}", n))
}

fn token(d: &DocView, t: i32) -> Result<&crate::fix::source::Piece, String> {
    d.tokens.get(t as usize).filter(|_| t >= 0).ok_or_else(|| format!("no token {}", t))
}

/// The token index range `[first, end)` of the tokens inside `span`.
fn tokens_in(d: &DocView, span: Span) -> (usize, usize) {
    let first = d.tokens.partition_point(|p| p.span.start < span.start);
    let end = d.tokens.partition_point(|p| p.span.end <= span.end).max(first);
    (first, end)
}

fn edit(it: &mut Interp<'_, '_>, a: &[Value], op: Op) -> R {
    let (gen, rule, group, tok, s) = (int(a, 0), int(a, 1), int(a, 2), int(a, 3), text(a, 4));
    on_host(it, |h| {
        match &h.doc {
            Some(d) if d.gen as i32 == gen && tok >= 0 && (tok as usize) < d.tokens.len() => {}
            _ => return Err("a token of another document than the one the rule reads".to_string()),
        }
        h.results.edits.push(Edit { rule: rule as u32, group: group as u32, op, token: tok as u32, text: s });
        Ok(Value::Unit)
    })
}

/// A Scala `Array` of `items`: the rules `fix::engine` hands the rule program.
pub fn array_of(items: Vec<Value>) -> Value {
    Value::Array(ArrayCell::new(items))
}

/// The items of an `Array`, none of another value.
pub fn array_items(v: &Value) -> Vec<Value> {
    match v {
        Value::Array(a) => a.borrow().clone(),
        _ => Vec::new(),
    }
}

impl Interp<'_, '_> {
    /// A new instance of the class `c` by its constructor of no arguments: `teq fix`'s project rules (`fix::engine`).
    pub fn new_instance(&mut self, c: crate::types::ClassId) -> Result<Value, super::Failure> {
        self.exec.depth = 0;
        let r = self.construct_new(c, Vec::new(), &super::Frame::new(None));
        self.finish(r)
    }
}

thread_local! {
    /// Whether this thread's interpreters answer `teq.fix.Native`: `teq fix`'s rule thread's alone, so that no other
    /// thread's builtin table carries what the fix verb alone needs (`builtins::table`).
    static WANTED: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

/// Makes the builtins of `teq.fix.Native` part of this thread's table, before its first interpreter is made.
pub fn want() {
    WANTED.with(|w| w.set(true));
}

pub(super) fn wanted() -> bool {
    WANTED.with(|w| w.get())
}

pub(super) fn install(t: &mut Table) {
    reg!(t, "teq.fix.Native.kindNames", |_it, _a| Ok(strings(KIND_NAMES.iter().map(|s| s.to_string()))));
    reg!(t, "teq.fix.Native.text", |it, a| on_doc(it, int(a, 0), |d| Ok(Value::str(&d.text))));
    reg!(t, "teq.fix.Native.path", |it, a| on_doc(it, int(a, 0), |d| Ok(Value::str(&d.path))));
    reg!(t, "teq.fix.Native.slice", |it, a| {
        let (s, e) = (int(a, 1).max(0) as u32, int(a, 2).max(0) as u32);
        on_doc(it, int(a, 0), |d| {
            let (bs, be) = (d.utf16.bytes(s).min(d.text.len() as u32) as usize, d.utf16.bytes(e).min(d.text.len() as u32) as usize);
            d.text.get(bs..be.max(bs)).map(Value::str).ok_or_else(|| format!("no text at {}..{}", s, e))
        })
    });
    reg!(t, "teq.fix.Native.root", |it, a| on_doc(it, int(a, 0), |d| Ok(Value::Int(d.tree.nodes.len() as i32 - 1))));
    reg!(t, "teq.fix.Native.kind", |it, a| {
        let n = int(a, 1);
        on_doc(it, int(a, 0), |d| Ok(Value::Int(node(d, n)?.kind as i32)))
    });
    reg!(t, "teq.fix.Native.start", |it, a| {
        let n = int(a, 1);
        on_doc(it, int(a, 0), |d| Ok(Value::Int(d.utf16.units(node(d, n)?.span.start) as i32)))
    });
    reg!(t, "teq.fix.Native.end", |it, a| {
        let n = int(a, 1);
        on_doc(it, int(a, 0), |d| Ok(Value::Int(d.utf16.units(node(d, n)?.span.end) as i32)))
    });
    reg!(t, "teq.fix.Native.parent", |it, a| {
        let n = int(a, 1);
        on_doc(it, int(a, 0), |d| {
            let p = node(d, n)?.parent;
            Ok(Value::Int(if p == NONE { -1 } else { p as i32 }))
        })
    });
    reg!(t, "teq.fix.Native.children", |it, a| {
        let n = int(a, 1);
        on_doc(it, int(a, 0), |d| {
            node(d, n)?;
            Ok(ints(d.tree.children(n as u32).iter().map(|&(_, c)| c as i32)))
        })
    });
    reg!(t, "teq.fix.Native.field", |it, a| {
        let (n, name) = (int(a, 1), text(a, 2));
        on_doc(it, int(a, 0), |d| {
            node(d, n)?;
            let f = FIELD_NAMES.iter().position(|f| *f == name).ok_or_else(|| format!("no field {}", name))?;
            Ok(ints(d.tree.children(n as u32).iter().filter(|&&(g, _)| g as usize == f).map(|&(_, c)| c as i32)))
        })
    });
    reg!(t, "teq.fix.Native.value", |it, a| {
        let n = int(a, 1);
        on_doc(it, int(a, 0), |d| {
            node(d, n)?;
            Ok(d.tree.value(n as u32).map_or(Value::Null, Value::str))
        })
    });
    reg!(t, "teq.fix.Native.flags", |it, a| {
        let n = int(a, 1);
        on_doc(it, int(a, 0), |d| Ok(Value::Int(node(d, n)?.flags as i32)))
    });
    reg!(t, "teq.fix.Native.nodeTokens", |it, a| {
        let n = int(a, 1);
        on_doc(it, int(a, 0), |d| {
            let (first, end) = tokens_in(d, node(d, n)?.span);
            Ok(ints([first as i32, end as i32]))
        })
    });
    reg!(t, "teq.fix.Native.tokenCount", |it, a| on_doc(it, int(a, 0), |d| Ok(Value::Int(d.tokens.len() as i32))));
    reg!(t, "teq.fix.Native.tokenKind", |it, a| {
        let k = int(a, 1);
        on_doc(it, int(a, 0), |d| Ok(Value::str(crate::fix::source::kind_name(token(d, k)?.kind))))
    });
    // DisableSyntax's walks (fix-speed's table, rows 8.1 and 8.3): the nodes of the kinds given, in the tree's pre-order
    // (`Tree.collect`'s: a node, then its children in order), and the tokens of the codes given, in the stream's order.
    reg!(t, "teq.fix.Native.preorder", |it, a| {
        let kinds = int_items(a, 1);
        on_doc(it, int(a, 0), |d| {
            let want = wanted_codes(kinds, KIND_NAMES.len());
            let mut out = Vec::new();
            let mut stack: Vec<u32> = d.tree.nodes.len().checked_sub(1).map(|root| root as u32).into_iter().collect();
            while let Some(n) = stack.pop() {
                if want[d.tree.nodes[n as usize].kind as usize] {
                    out.push(n as i32);
                }
                stack.extend(d.tree.children(n).iter().rev().map(|&(_, c)| c));
            }
            Ok(ints(out))
        })
    });
    // The subtree of a node in pre-order, the node first (`Tree.collect`'s and `Tree.traverse`'s order).
    reg!(t, "teq.fix.Native.subtree", |it, a| {
        let n = int(a, 1);
        on_doc(it, int(a, 0), |d| {
            node(d, n)?;
            let mut out = Vec::new();
            let mut stack = vec![n as u32];
            while let Some(m) = stack.pop() {
                out.push(m as i32);
                stack.extend(d.tree.children(m).iter().rev().map(|&(_, c)| c));
            }
            Ok(ints(out))
        })
    });
    reg!(t, "teq.fix.Native.tokensOf", |it, a| {
        let codes = int_items(a, 1);
        on_doc(it, int(a, 0), |d| {
            let want = wanted_codes(codes, crate::fix::source::TOKEN_KINDS.len());
            Ok(ints(d.tokens.iter().enumerate().filter(|(_, p)| want[crate::fix::source::kind_code(p.kind) as usize]).map(|(i, _)| i as i32)))
        })
    });
    reg!(t, "teq.fix.Native.tokenKindNames", |_it, _a| Ok(strings(crate::fix::source::TOKEN_KINDS.iter().map(|s| s.to_string()))));
    reg!(t, "teq.fix.Native.tokenCodes", |it, a| on_doc(it, int(a, 0), |d| Ok(ints(d.tokens.iter().map(|p| crate::fix::source::kind_code(p.kind) as i32)))));
    reg!(t, "teq.fix.Native.tokenStart", |it, a| {
        let k = int(a, 1);
        on_doc(it, int(a, 0), |d| Ok(Value::Int(d.utf16.units(token(d, k)?.span.start) as i32)))
    });
    reg!(t, "teq.fix.Native.tokenEnd", |it, a| {
        let k = int(a, 1);
        on_doc(it, int(a, 0), |d| Ok(Value::Int(d.utf16.units(token(d, k)?.span.end) as i32)))
    });
    reg!(t, "teq.fix.Native.line", |it, a| {
        let at = int(a, 1).max(0) as u32;
        on_doc(it, int(a, 0), |d| {
            let byte = d.utf16.bytes(at).min(d.text.len() as u32) as usize;
            Ok(Value::Int(d.text.as_bytes()[..byte].iter().filter(|&&b| b == b'\n').count() as i32))
        })
    });
    reg!(t, "teq.fix.Native.column", |it, a| {
        let at = int(a, 1).max(0) as u32;
        on_doc(it, int(a, 0), |d| {
            let byte = d.utf16.bytes(at).min(d.text.len() as u32) as usize;
            let line_start = d.text.as_bytes()[..byte].iter().rposition(|&b| b == b'\n').map_or(0, |p| p + 1);
            Ok(Value::Int((at - d.utf16.units(line_start as u32)) as i32))
        })
    });
    reg!(t, "teq.fix.Native.unusedImports", |it, a| {
        on_doc(it, int(a, 0), |d| match &d.unused {
            Ok(nodes) => Ok(ints(nodes.iter().map(|&n| n as i32))),
            Err(why) => Err(format!("the unused imports of {} are unavailable: {}", d.path, why)),
        })
    });
    reg!(t, "teq.fix.Native.expanded", |it, a| on_doc(it, int(a, 0), |d| Ok(Value::Bool(d.expanded))));
    reg!(t, "teq.fix.Native.docSymbols", |it, a| {
        query(it, int(a, 0), |_| Ok(Ask::Doc), |r| match r {
            Answer::Doc(b) => Ok(bytes(b)),
            _ => Err(other("docSymbols")),
        })
    });
    reg!(t, "teq.fix.Native.symbolsAt", |it, a| {
        let (start, end) = (int(a, 1), int(a, 2));
        query(
            it,
            int(a, 0),
            |d| {
                let units = d.utf16.units(d.text.len() as u32) as i32;
                if start < 0 || end < start || end > units {
                    return Err(format!("no range [{}, {}) of {}", start, end, d.path));
                }
                Ok(Ask::Symbols { start: d.utf16.bytes(start as u32), end: d.utf16.bytes(end as u32) })
            },
            |r| match r {
                Answer::Symbols(v) => Ok(strings(v)),
                _ => Err(other("symbolsAt")),
            },
        )
    });
    reg!(t, "teq.fix.Native.info", |it, a| {
        let symbol = text(a, 1);
        query(it, int(a, 0), |_| Ok(Ask::Info { symbol }), |r| match r {
            Answer::Info(Some(b)) => Ok(bytes(b)),
            Answer::Info(None) => Ok(Value::Null),
            _ => Err(other("info")),
        })
    });
    reg!(t, "teq.fix.Native.addLeft", |it, a| edit(it, a, Op::Left));
    reg!(t, "teq.fix.Native.addRight", |it, a| edit(it, a, Op::Right));
    reg!(t, "teq.fix.Native.removeToken", |it, a| edit(it, a, Op::Remove));
    reg!(t, "teq.fix.Native.removeImportee", |it, a| {
        let (gen, rule, group, n) = (int(a, 0), int(a, 1), int(a, 2), int(a, 3));
        on_host(it, |h| {
            match &h.doc {
                Some(d) if d.gen as i32 == gen && n >= 0 && (n as usize) < d.tree.nodes.len() => {}
                _ => return Err("an importee of another document than the one the rule reads".to_string()),
            }
            h.results.imports.push(ImportOp::Remove { rule: rule as u32, group: group as u32, importee: n as u32 });
            Ok(Value::Unit)
        })
    });
    reg!(t, "teq.fix.Native.addGlobalImport", |it, a| {
        let (rule, group, importer) = (int(a, 1), int(a, 2), text(a, 3));
        on_host(it, |h| {
            h.results.imports.push(ImportOp::Add { rule: rule as u32, group: group as u32, importer });
            Ok(Value::Unit)
        })
    });
    reg!(t, "teq.fix.Native.lint", |it, a| {
        // The explanation (`a`'s sixth) is scalafix's `--explain`'s, which teq does not print.
        let (gen, rule, group, category, message, severity, start, end) = (int(a, 0), int(a, 1), int(a, 2), text(a, 3), text(a, 4), int(a, 6), int(a, 7), int(a, 8));
        on_host(it, |h| {
            let span = match &h.doc {
                Some(d) if d.gen as i32 == gen => {
                    let (s, e) = (start.max(0) as u32, end.max(start).max(0) as u32);
                    Span::new(d.utf16.bytes(s).min(d.text.len() as u32), d.utf16.bytes(e).min(d.text.len() as u32))
                }
                _ => return Err("a position of another document than the one the rule reads".to_string()),
            };
            let severity = match severity {
                0 => Severity::Info,
                1 => Severity::Warning,
                _ => Severity::Error,
            };
            h.results.lints.push(Lint { rule: rule as u32, category, message, severity, span, group: group as u32 });
            Ok(Value::Unit)
        })
    });
    reg!(t, "teq.fix.Native.newGroup", |it, _a| {
        on_host(it, |h| {
            h.groups += 1;
            Ok(Value::Int(h.groups as i32))
        })
    });
    reg!(t, "teq.fix.Native.failed", |it, a| {
        let (rule, msg) = (int(a, 1), text(a, 2));
        on_host(it, |h| {
            h.results.failures.push((rule as u32, msg));
            Ok(Value::Unit)
        })
    });
    reg!(t, "teq.fix.Native.isAnchor", |_it, a| Ok(Value::Bool(crate::fix::escape::anchor(&text(a, 0)).is_some())));
    // The configuration: nodes by index, the root 0; reading a field marks it read.
    reg!(t, "teq.fix.Native.confKind", |it, a| {
        let c = int(a, 0);
        on_host(it, |h| {
            let n = h.conf.nodes.get(c as usize).filter(|_| c >= 0).ok_or("no setting")?;
            Ok(Value::Int(match n.value {
                Conf::Str(_) => 0,
                Conf::Num(_) => 1,
                Conf::Bool(_) => 2,
                Conf::Null => 3,
                Conf::List(_) => 4,
                Conf::Obj(_) => 5,
            }))
        })
    });
    reg!(t, "teq.fix.Native.confString", |it, a| {
        let c = int(a, 0);
        on_host(it, |h| {
            conf_node(h, c, "confString")?;
            Ok(h.conf.str(c as u32).map_or(Value::Null, Value::string))
        })
    });
    reg!(t, "teq.fix.Native.confField", |it, a| {
        let (c, key) = (int(a, 0), text(a, 1));
        on_host(it, |h| {
            conf_node(h, c, "confField")?;
            let f = h.conf.field(c as u32, &key);
            if let Some(f) = f {
                mark_read(h, f);
            }
            Ok(Value::Int(f.map_or(-1, |f| f as i32)))
        })
    });
    reg!(t, "teq.fix.Native.confItems", |it, a| {
        let c = int(a, 0);
        on_host(it, |h| match &h.conf.nodes[conf_node(h, c, "confItems")?].value {
            Conf::List(items) => Ok(ints(items.iter().map(|&i| i as i32))),
            _ => Err("a setting that is no list read as one".to_string()),
        })
    });
    reg!(t, "teq.fix.Native.confKeys", |it, a| {
        let c = int(a, 0);
        on_host(it, |h| match &h.conf.nodes[conf_node(h, c, "confKeys")?].value {
            Conf::Obj(fields) => Ok(strings(fields.iter().map(|(k, _)| k.clone()))),
            _ => Ok(strings([])),
        })
    });
    reg!(t, "teq.fix.Native.confPlace", |it, a| {
        let c = int(a, 0);
        on_host(it, |h| {
            conf_node(h, c, "confPlace")?;
            Ok(Value::string(h.conf.place(c as u32)))
        })
    });
}

fn conf_node(h: &Host, c: i32, what: &str) -> Result<usize, String> {
    (c >= 0 && (c as usize) < h.conf.nodes.len()).then_some(c as usize).ok_or_else(|| format!("{}: no setting {}", what, c))
}

/// A setting and all it holds count as read once its key is.
fn mark_read(h: &mut Host, n: u32) {
    if h.read.len() < h.conf.nodes.len() {
        h.read.resize(h.conf.nodes.len(), false);
    }
    h.read[n as usize] = true;
    let children: Vec<u32> = match &h.conf.nodes[n as usize].value {
        Conf::List(items) => items.clone(),
        _ => Vec::new(),
    };
    for c in children {
        mark_read(h, c);
    }
}
