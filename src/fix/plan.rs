//! The patch planner (docs/FIX.md, "Patches"): what the rules returned for a document, filtered by its
//! suppressions, merged, checked and turned into byte edits of its text and its lints.
//!
//! - A rule's own edits of one token merge as scalafix's `PatchInternals.merge` merges them: additions concatenated in
//!   the order made, a removal and an addition a replacement, two removals one.
//! - Across rules, per token: rules whose edits of it are identical count once; additions of several rules to a
//!   token none removes are joined in the configured order of the rules; a token one rule removes and another adds to
//!   is the addition, as `merge` makes it, unless two rules add different text to it, a conflict (scalafix
//!   concatenates there; teq refuses, both rules named).
//! - `removeImportee`s are token removals, as scalafix's `ImportPatchOps` makes them of every rule's removed importees
//!   (`import_removals`), merged with the rules' token edits.
//! - The edits are applied as dotty's `Rewrites.Patches.apply` applies its patches: identical ones once, by their
//!   start, the untouched bytes copied; a `\n` a rule inserts is written as the file's line ending.

use super::escape::Escapes;
use super::host::{DocView, ImportOp, Lint, Op, Results, Severity};
use super::source::Kind;
use super::nodes::{is_importee, F, K, NONE};
use crate::source::Span;
use crate::token::Tok;
use std::collections::BTreeMap;

/// A lint as printed: its rule's name and category, message, severity and span.
#[derive(Clone, Debug)]
pub struct Printed {
    pub id: String,
    pub message: String,
    pub severity: Severity,
    pub span: Span,
}

/// A document's plan.
#[derive(Default, Debug)]
pub struct Plan {
    /// Byte edits of the text, sorted, none overlapping, each with the rules it comes from.
    pub edits: Vec<(Span, String, Vec<u32>)>,
    pub lints: Vec<Printed>,
    /// What refuses the run: conflicts and rules' failures, as messages.
    pub refused: Vec<String>,
}

/// The full id of a lint: its rule's name and its category (`LintID.fullID`).
pub fn full_id(rule: &str, category: &str) -> String {
    if category.is_empty() {
        rule.to_string()
    } else {
        format!("{}.{}", rule, category)
    }
}

pub fn plan(d: &DocView, r: &Results, names: &[String]) -> Plan {
    let mut out = Plan::default();
    for (rule, msg) in &r.failures {
        out.refused.push(format!("{}: the rule {} failed: {}", d.path, names[*rule as usize], msg));
    }
    let mut escapes = Escapes::of(d);
    let mut removals: Vec<(u32, u32, u32)> = Vec::new();
    for op in &r.imports {
        if let ImportOp::Remove { rule, group, importee } = *op {
            if (importee as usize) < d.tree.nodes.len() && is_importee(d.tree.nodes[importee as usize].kind) {
                removals.push((rule, group, importee));
            } else {
                out.refused.push(format!("{}: the rule {} removes no importee of the file", d.path, names[rule as usize]));
            }
        }
    }
    // Atomic groups with an edit where their rule is suppressed go whole (`EscapeHatch.filter`'s `AtomicPatch`, which
    // checks the token edits it applies, `treePatchApply` over the group alone, `atomic_positions`).
    let mut dropped: Vec<u32> = Vec::new();
    if !escapes.is_empty() {
        let mut groups: Vec<u32> = Vec::new();
        let op_groups = r.imports.iter().map(|op| match op {
            ImportOp::Remove { group, .. } | ImportOp::Add { group, .. } => *group,
        });
        for g in op_groups.chain(r.edits.iter().map(|e| e.group)) {
            if g != 0 && !groups.contains(&g) {
                groups.push(g);
            }
        }
        let written = written_importers(d);
        for g in groups {
            let own = removed_by(d, r, |group| group == g);
            for (rule, at) in atomic_positions(d, r, g, &written, &removals, &own) {
                if !escapes.enabled(&names[rule as usize], at) {
                    dropped.push(g);
                    break;
                }
            }
        }
    }
    for l in &r.lints {
        if l.group != 0 && dropped.contains(&l.group) {
            continue;
        }
        let id = full_id(&names[l.rule as usize], &l.category);
        if escapes.is_empty() || escapes.enabled(&id, l.span.start) {
            out.lints.push(printed(l, id));
        }
    }
    if !escapes.is_empty() {
        for span in escapes.unused() {
            out.lints.push(Printed { id: "UnusedScalafixSuppression".to_string(), message: "Unused Scalafix suppression, this can be removed".to_string(), severity: Severity::Warning, span });
        }
    }
    // The tokens the edits that survive suppression remove, which the import conversion reads.
    let removed_by_rules = removed_by(d, r, |group| group == 0 || !dropped.contains(&group));
    let ending = line_ending(&d.text);
    // Per token, per rule, the rule's merged edit: removed, text left of it, text right of it.
    let mut per_token: BTreeMap<u32, BTreeMap<u32, (bool, String, String)>> = BTreeMap::new();
    let kept: Vec<(u32, u32)> = removals.iter().filter(|&&(_, g, _)| g == 0 || !dropped.contains(&g)).map(|&(rule, _, n)| (rule, n)).collect();
    for (tok, rules) in import_removals(d, &kept, &removed_by_rules) {
        for rule in rules {
            per_token.entry(tok).or_default().entry(rule).or_insert((false, String::new(), String::new())).0 = true;
        }
    }
    for e in r.edits.iter().filter(|e| e.group == 0 || !dropped.contains(&e.group)) {
        let entry = per_token.entry(e.token).or_default().entry(e.rule).or_insert((false, String::new(), String::new()));
        match e.op {
            Op::Remove => entry.0 = true,
            Op::Left => entry.1.push_str(&e.text),
            Op::Right => entry.2.push_str(&e.text),
        }
    }
    let mut edits: Vec<(Span, String, Vec<u32>)> = Vec::new();
    for (&tok, by_rule) in &per_token {
        // Rules whose edits of the token are the same count once, the first in the configured order.
        let mut distinct: Vec<(Vec<u32>, (bool, String, String))> = Vec::new();
        for (&rule, edit) in by_rule {
            match distinct.iter_mut().find(|(_, e)| e == edit) {
                Some((rules, _)) => rules.push(rule),
                None => distinct.push((vec![rule], edit.clone())),
            }
        }
        let span = d.tokens[tok as usize].span;
        let removed = distinct.iter().any(|(_, e)| e.0);
        let mut adders = distinct.iter().filter(|(_, e)| !e.1.is_empty() || !e.2.is_empty());
        let first = adders.next();
        let other = first.and_then(|(_, a)| adders.find(|(_, b)| (&a.1, &a.2) != (&b.1, &b.2)));
        if let (true, Some((a, _)), Some((b, _))) = (removed, first, other) {
            out.refused.push(conflict(d, span, names, a[0], b[0]));
            continue;
        }
        // The added text's `\n`s written as the file's line ending; the token's own bytes copied as they are (a
        // multiline string's line breaks are its value: `Scanners.getRawStringLit` keeps them).
        let mut text = String::new();
        for (_, e) in &distinct {
            text.push_str(&with_ending(&e.1, ending));
        }
        if !removed {
            text.push_str(&d.text[span.start as usize..span.end as usize]);
        }
        for (_, e) in &distinct {
            text.push_str(&with_ending(&e.2, ending));
        }
        let rules: Vec<u32> = distinct.iter().flat_map(|(r, _)| r.iter().copied()).collect();
        edits.push((span, text, rules));
    }
    let adds: Vec<(u32, &str)> = r
        .imports
        .iter()
        .filter_map(|op| match op {
            ImportOp::Add { rule, group, importer } if *group == 0 || !dropped.contains(group) => Some((*rule, importer.as_str())),
            _ => None,
        })
        .collect();
    if let Some(e) = global_imports(d, &adds, ending) {
        edits.push(e);
    }
    // `Rewrites.Patches.apply`: the patches as collected (the tokens' in their order, then the added imports), each
    // distinct one once, sorted by their start alone (a stable sort), none overlapping the next.
    let mut distinct: Vec<(Span, String, Vec<u32>)> = Vec::with_capacity(edits.len());
    for e in edits {
        if !distinct.iter().any(|(s, t, _)| *s == e.0 && *t == e.1) {
            distinct.push(e);
        }
    }
    let mut edits = distinct;
    edits.sort_by_key(|(s, _, _)| s.start);
    for w in edits.windows(2) {
        if w[0].0.end > w[1].0.start {
            out.refused.push(conflict(d, w[1].0, names, w[0].2[0], w[1].2[0]));
        }
    }
    out.edits = edits;
    out.lints.sort_by_key(|l| (l.span.start, l.span.end));
    out
}

/// The positions `EscapeHatch.filter` checks of the atomic group `g` (its `AtomicPatch` case: the token patches its
/// patch is applied as, `treePatchApply`), each with its rule: each importer the group adds, unless written or added
/// already, at the edit token it goes right of (`ImportPatchOps`' `extraPatches`); the token removals of the importees
/// it removes, converted without the other groups' (`removed`, the tokens its own edits remove); then its token edits.
fn atomic_positions(d: &DocView, r: &Results, g: u32, written: &[&str], removals: &[(u32, u32, u32)], removed: &[bool]) -> Vec<(u32, u32)> {
    let mut out: Vec<(u32, u32)> = Vec::new();
    let mut added: Vec<&str> = Vec::new();
    for op in &r.imports {
        if let ImportOp::Add { rule, group, importer } = op {
            if *group != g || written.contains(&importer.as_str()) || added.contains(&importer.as_str()) {
                continue;
            }
            added.push(importer);
            out.push((*rule, import_token_start(d)));
        }
    }
    let own: Vec<(u32, u32)> = removals.iter().filter(|&&(_, h, _)| h == g).map(|&(rule, _, n)| (rule, n)).collect();
    for (tok, rules) in import_removals(d, &own, removed) {
        out.push((rules[0], d.tokens[tok as usize].span.start));
    }
    out.extend(r.edits.iter().filter(|e| e.group == g).map(|e| (e.rule, d.tokens[e.token as usize].span.start)));
    out
}

/// The tokens the removals of the edits of the groups `keep` holds remove.
fn removed_by(d: &DocView, r: &Results, keep: impl Fn(u32) -> bool) -> Vec<bool> {
    let mut removed = vec![false; d.tokens.len()];
    for e in r.edits.iter().filter(|e| matches!(e.op, Op::Remove) && keep(e.group)) {
        removed[e.token as usize] = true;
    }
    removed
}

fn printed(l: &Lint, id: String) -> Printed {
    Printed { id, message: l.message.clone(), severity: l.severity, span: l.span }
}

fn conflict(d: &DocView, span: Span, names: &[String], a: u32, b: u32) -> String {
    let (line, col, _) = crate::source::locate(&d.text, span.start as usize);
    format!("{}:{}:{}: the edits of {} and {} overlap", d.path, line, col, names[a as usize], names[b as usize])
}

/// The file's line ending: that of its first line break, `\n` for a file without one.
pub fn line_ending(text: &str) -> &'static str {
    match text.find('\n') {
        Some(i) if i > 0 && text.as_bytes()[i - 1] == b'\r' => "\r\n",
        _ => "\n",
    }
}

fn with_ending(text: &str, ending: &str) -> String {
    if ending == "\n" || !text.contains('\n') {
        return text.to_string();
    }
    let mut out = String::with_capacity(text.len() + 8);
    let b = text.as_bytes();
    for (i, c) in text.char_indices() {
        if c == '\n' && (i == 0 || b[i - 1] != b'\r') {
            out.push_str("\r\n");
        } else {
            out.push(c);
        }
    }
    out
}

/// The token removals of the importees `removals` names, each with the rules that removed what it answers to, as
/// scalafix's `superNaiveImportPatchToTokenPatchConverter` makes them (`ImportPatchOps.scala` at v0.14.7): each removed
/// importee's tokens, and each removed importer's of an import (one whose importees all go), with the comma before it
/// and, where it ends the leading run of removed ones beside others, the comma after it (`remove`); the braces of an
/// importer left with one name or wildcard (`curlyBraceRemoves`); and an import whose importers all go
/// (`importsRemoves`, `deleted_import`). A hiding selector (`a as _`) whose importer loses its last wildcard goes with
/// it, as dotty's `CheckUnused.matchingSelector` counts it used only beside one.
fn import_removals(d: &DocView, removals: &[(u32, u32)], removed_by_rules: &[bool]) -> BTreeMap<u32, Vec<u32>> {
    let t = &d.tree;
    let kind = |n: u32| t.nodes[n as usize].kind;
    let field = |n: u32, f: F| -> Vec<u32> { t.children(n).iter().filter(|c| c.0 == f).map(|c| c.1).collect() };
    let mut rules_of: BTreeMap<u32, Vec<u32>> = BTreeMap::new();
    for &(rule, n) in removals {
        let rules = rules_of.entry(n).or_default();
        if !rules.contains(&rule) {
            rules.push(rule);
        }
    }
    let mut out = Removals { d, out: BTreeMap::new() };
    if rules_of.is_empty() {
        return out.out;
    }
    let imports: Vec<u32> = (0..t.nodes.len() as u32).filter(|&n| kind(n) == K::Import).collect();
    for importer in imports.iter().flat_map(|&i| field(i, F::Importers)) {
        let importees = field(importer, F::Importees);
        let wildcards: Vec<u32> = importees.iter().copied().filter(|&m| kind(m) == K::ImporteeWildcard).collect();
        let rules: Vec<u32> = wildcards.iter().filter_map(|m| rules_of.get(m)).flatten().copied().collect();
        if rules.is_empty() || wildcards.iter().any(|m| !rules_of.contains_key(m)) {
            continue;
        }
        for &m in &importees {
            if kind(m) == K::ImporteeUnimport {
                rules_of.entry(m).or_insert_with(|| rules.clone());
            }
        }
    }
    let gone = |n: u32| rules_of.contains_key(&n);
    let rules_in = |ns: &[u32]| -> Vec<u32> {
        let mut rules: Vec<u32> = ns.iter().filter_map(|n| rules_of.get(n)).flatten().copied().collect();
        rules.sort_unstable();
        rules.dedup();
        rules
    };
    let mut gone_importers: Vec<(u32, Vec<u32>)> = Vec::new();
    for importer in imports.iter().flat_map(|&i| field(i, F::Importers)) {
        let importees = field(importer, F::Importees);
        let kept: Vec<u32> = importees.iter().copied().filter(|&m| !gone(m)).collect();
        if kept.is_empty() {
            gone_importers.push((importer, rules_in(&importees)));
        } else if kept.len() == 1 && importees.len() > 1 && matches!(kind(kept[0]), K::ImporteeWildcard | K::ImporteeName) {
            out.braces(t.nodes[importer as usize].span, &rules_in(&importees));
        }
    }
    for (&n, rules) in &rules_of {
        let siblings = field(t.nodes[n as usize].parent, F::Importees);
        out.remove(t.nodes[n as usize].span, responsible(n, &siblings, &gone), rules);
    }
    // Where scalafix's removals leave a comma alone between an importer's braces (`removeUpToFirstComma` stopped by a
    // comment, `a, // c` before a removed `b`), a text that does not parse, dotty's `checkImports` edit instead: each
    // removed importee with what follows it to the next, the last from the comma after the importee kept before it.
    for importer in imports.iter().flat_map(|&i| field(i, F::Importers)) {
        let importees = field(importer, F::Importees);
        let kept: Vec<usize> = (0..importees.len()).filter(|&k| !gone(importees[k])).collect();
        if kept.is_empty() || kept.len() == importees.len() {
            continue;
        }
        let Some((open, close)) = out.braces_of(t.nodes[importer as usize].span) else { continue };
        if !out.comma_alone(open, close) {
            continue;
        }
        out.out.retain(|&tok, _| tok as usize <= open || tok as usize >= close);
        let first = |out: &Removals, n: u32| out.within(t.nodes[n as usize].span).start;
        for k in (0..importees.len()).filter(|k| !kept.contains(k)) {
            let rules = rules_of.get(&importees[k]).cloned().unwrap_or_default();
            if k + 1 < importees.len() {
                let (from, to) = (first(&out, importees[k]), first(&out, importees[k + 1]));
                (from..to).for_each(|i| out.take(i, &rules));
            } else if let Some(&prev) = kept.iter().rev().find(|&&p| p < k) {
                let after = out.within(t.nodes[importees[prev] as usize].span).end;
                if let Some(comma) = (after..close).find(|&i| matches!(out.kind(i), Kind::Tok(Tok::Comma))) {
                    let end = out.within(t.nodes[importees[k] as usize].span).end;
                    (comma..first(&out, importees[prev + 1])).chain(first(&out, importees[k])..end).for_each(|i| out.take(i, &rules));
                }
            }
        }
        if kept.len() == 1 && matches!(kind(importees[kept[0]]), K::ImporteeWildcard | K::ImporteeName) {
            out.braces(t.nodes[importer as usize].span, &rules_in(&importees));
        }
    }
    let importer_gone = |n: u32| gone_importers.iter().any(|(m, _)| *m == n);
    for (n, rules) in &gone_importers {
        let siblings = field(t.nodes[*n as usize].parent, F::Importers);
        out.remove(t.nodes[*n as usize].span, responsible(*n, &siblings, &importer_gone), rules);
    }
    for &i in &imports {
        let importers = field(i, F::Importers);
        if importers.iter().all(|&n| importer_gone(n)) {
            out.deleted_import(t.nodes[i as usize].span, removed_by_rules, &rules_in(&importers.iter().flat_map(|&n| field(n, F::Importees)).collect::<Vec<u32>>()));
        }
    }
    out.out
}

/// `remove`'s `isResponsibleForTrailingComma`: `n` ends the leading run of removed ones among several `siblings`.
fn responsible(n: u32, siblings: &[u32], gone: &dyn Fn(u32) -> bool) -> bool {
    siblings.len() > 1 && siblings.iter().take_while(|&&s| gone(s)).last() == Some(&n)
}

/// Token removals, each with its rules.
struct Removals<'a> {
    d: &'a DocView,
    out: BTreeMap<u32, Vec<u32>>,
}

impl Removals<'_> {
    fn take(&mut self, tok: usize, rules: &[u32]) {
        let at = self.out.entry(tok as u32).or_default();
        for &r in rules {
            if !at.contains(&r) {
                at.push(r);
            }
        }
    }

    /// The indices of the tokens within `span`.
    fn within(&self, span: Span) -> std::ops::Range<usize> {
        let tokens = &self.d.tokens;
        tokens.partition_point(|p| p.span.start < span.start)..tokens.partition_point(|p| p.span.start < span.end)
    }

    fn kind(&self, i: usize) -> Kind {
        self.d.tokens[i].kind
    }

    /// `remove`: the tree's tokens, the whitespace and comma before them, and where `trailing` the whitespace and
    /// comma after them.
    fn remove(&mut self, span: Span, trailing: bool, rules: &[u32]) {
        let toks = self.within(span);
        for i in toks.clone() {
            self.take(i, rules);
        }
        self.up_to_first_comma((0..toks.start).rev(), rules);
        if trailing {
            self.up_to_first_comma(toks.end..self.d.tokens.len(), rules);
        }
    }

    /// `removeUpToFirstComma`: the whitespace and the first comma of `toks` up to another token, where a comma is met.
    fn up_to_first_comma(&mut self, toks: impl Iterator<Item = usize>, rules: &[u32]) {
        let mut met: Vec<usize> = Vec::new();
        let mut comma = false;
        for i in toks {
            match self.kind(i) {
                k if is_whitespace(k) => met.push(i),
                Kind::Tok(Tok::Comma) if !comma => {
                    comma = true;
                    met.push(i);
                }
                _ => break,
            }
        }
        if comma {
            for i in met {
                self.take(i, rules);
            }
        }
    }

    /// The token indices of the importer's braces over `importer`, where it has them.
    fn braces_of(&self, importer: Span) -> Option<(usize, usize)> {
        let toks = self.within(importer);
        let open = toks.clone().find(|&i| matches!(self.kind(i), Kind::Tok(Tok::LBrace)))?;
        let mut depth = 0;
        let close = (open..self.d.tokens.len()).find(|&i| {
            match self.kind(i) {
                Kind::Tok(Tok::LBrace) => depth += 1,
                Kind::Tok(Tok::RBrace) => depth -= 1,
                _ => {}
            }
            depth == 0
        })?;
        Some((open, close))
    }

    /// The tokens kept between the braces `open` and `close` hold a comma with no importee before it, or one after the
    /// last importee that is no trailing comma: Scala's, as dotty's `Scanners` reads it, is followed by the closing
    /// brace a line further, comments between, so both braces must stay.
    fn comma_alone(&self, open: usize, close: usize) -> bool {
        let mut item = false;
        let mut commas = 0;
        let mut line_after = false;
        for i in open + 1..close {
            if self.out.contains_key(&(i as u32)) {
                continue;
            }
            match self.kind(i) {
                k if is_line_end(k) => line_after = true,
                Kind::Comment => {
                    let s = self.d.tokens[i].span;
                    line_after |= self.d.text[s.start as usize..s.end as usize].contains('\n');
                }
                k if is_whitespace(k) => {}
                Kind::Tok(Tok::Comma) if !item => return true,
                Kind::Tok(Tok::Comma) => {
                    item = false;
                    commas += 1;
                    line_after = false;
                }
                _ => item = true,
            }
        }
        let braced = !self.out.contains_key(&(open as u32)) && !self.out.contains_key(&(close as u32));
        commas > 0 && !item && !(line_after && braced)
    }

    /// `removeSpaces`: the whitespace and commas of `toks` up to another token.
    fn spaces(&mut self, toks: impl Iterator<Item = usize>, rules: &[u32]) {
        for i in toks {
            if !is_whitespace(self.kind(i)) && !matches!(self.kind(i), Kind::Tok(Tok::Comma)) {
                break;
            }
            self.take(i, rules);
        }
    }

    /// `curlyBraceRemoves`: the importer's first `{` and the whitespace and commas after it, its matching `}` and the
    /// whitespace and commas before it.
    fn braces(&mut self, importer: Span, rules: &[u32]) {
        let toks = self.within(importer);
        let Some(open) = toks.clone().find(|&i| matches!(self.kind(i), Kind::Tok(Tok::LBrace))) else { return };
        let mut depth = 0;
        let close = (open..self.d.tokens.len()).find(|&i| {
            match self.kind(i) {
                Kind::Tok(Tok::LBrace) => depth += 1,
                Kind::Tok(Tok::RBrace) => depth -= 1,
                _ => {}
            }
            depth == 0
        });
        let Some(close) = close else { return };
        self.take(open, rules);
        self.spaces(open + 1..self.d.tokens.len(), rules);
        self.take(close, rules);
        self.spaces((0..close).rev(), rules);
    }

    /// An import removed whole: `importsRemoves`' tokens (its own, the spaces and tabs before it and the line break
    /// before those) and the bytes dotty's `editPosAt` deletes with it (`unused_edits::deletion_at`: its indentation,
    /// the blanks after it, and where its line is otherwise blank the line's break, with the blank line before it where
    /// one stands on each side). A deleted line loses one line break of the two around it, the one another rule's edit
    /// removes where exactly one is, else the one before (scalafix's); the blank line before it is dotty's.
    fn deleted_import(&mut self, span: Span, removed_by_rules: &[bool], rules: &[u32]) {
        let toks = self.within(span);
        let mut begin = toks.start;
        while begin > 0 && matches!(self.kind(begin - 1), Kind::Space | Kind::Tab) {
            begin -= 1;
        }
        let before = (begin > 0 && is_line_end(self.kind(begin - 1))).then(|| begin - 1);
        let (start, end) = crate::typer::unused_edits::deletion_at(&self.d.text, span.start as usize, span.end as usize);
        let dotty: Vec<usize> = self.within(Span::new(start as u32, end as u32)).filter(|&i| self.d.tokens[i].span.end as usize <= end).collect();
        let after = dotty.iter().copied().find(|&i| self.d.tokens[i].span.start >= span.end && is_line_end(self.kind(i)));
        let spare = match (before, after) {
            (Some(b), Some(a)) if !dotty.contains(&b) => Some(if removed_by_rules[a] && !removed_by_rules[b] { b } else { a }),
            _ => None,
        };
        for i in (before.unwrap_or(begin)..toks.end).chain(dotty).filter(|&i| Some(i) != spare) {
            self.take(i, rules);
        }
    }
}

fn is_whitespace(k: Kind) -> bool {
    matches!(k, Kind::Space | Kind::Tab) || is_line_end(k)
}

/// The imports the rules add (`Patch.addGlobalImport`), as scalafix's `ImportPatchOps` adds them
/// (`superNaiveImportPatchToTokenPatchConverter`): an importer is added unless the file has it written in a top-level
/// import or it was added already, the importers sorted by their prefix (a stable sort), each `\n` and
/// `import <importer>` added right of the edit token: the last token of the file's leading imports
/// (`extractImports`), else `fallbackToken`'s. Two departures, so that a second run adds nothing (teq's round two
/// refuses a rule that rewrites its own output): scalafix counts the imports of a package clause alone as written
/// (`importer.parent.parent.is[Pkg]`), teq a source's too; and the importers are not grouped by their prefix, as
/// scalafix writes them with `groupImportsByPrefix = false` (a grouped `a.{b, c}` is no `a.b` written).
fn global_imports(d: &DocView, adds: &[(u32, &str)], ending: &str) -> Option<(Span, String, Vec<u32>)> {
    let written = written_importers(d);
    let mut added: Vec<(&str, Vec<u32>)> = Vec::new();
    for &(rule, importer) in adds {
        if written.contains(&importer) {
            continue;
        }
        match added.iter_mut().find(|(s, _)| *s == importer) {
            Some((_, rules)) => {
                if !rules.contains(&rule) {
                    rules.push(rule);
                }
            }
            None => added.push((importer, vec![rule])),
        }
    }
    if added.is_empty() {
        return None;
    }
    added.sort_by(|a, b| prefix(a.0).cmp(prefix(b.0)));
    let mut text = String::new();
    for (importer, _) in &added {
        text.push_str("\nimport ");
        text.push_str(importer);
    }
    let at = import_edit_end(d);
    let mut rules: Vec<u32> = Vec::new();
    for r in added.iter().flat_map(|(_, r)| r.iter().copied()) {
        if !rules.contains(&r) {
            rules.push(r);
        }
    }
    Some((Span::new(at, at), with_ending(&text, ending), rules))
}

/// The importers of the file's top-level imports as written, which an added import of the same syntax does not repeat.
fn written_importers(d: &DocView) -> Vec<&str> {
    let t = &d.tree;
    let kind = |n: u32| t.nodes[n as usize].kind;
    let parent = |n: u32| t.nodes[n as usize].parent;
    let top_level = |n: u32| n != NONE && matches!(kind(n), K::PkgBody | K::Source);
    (0..t.nodes.len() as u32)
        .filter(|&n| kind(n) == K::Importer && parent(n) != NONE && kind(parent(n)) == K::Import && top_level(parent(parent(n))))
        .map(|n| &d.text[t.nodes[n as usize].span.start as usize..t.nodes[n as usize].span.end as usize])
        .collect()
}

/// Where an added import goes: the end of the edit token, the last token of the file's leading imports, else
/// `fallbackToken`'s.
fn import_edit_end(d: &DocView) -> u32 {
    match leading_imports(d).last() {
        Some(&last) => d.tree.nodes[last as usize].span.end,
        None => fallback_end(d),
    }
}

/// The start of that edit token, 0 for scalameta's `BOF`.
fn import_token_start(d: &DocView) -> u32 {
    let at = import_edit_end(d);
    d.tokens.iter().find(|p| p.span.end == at && p.span.start < at).map_or(at, |p| p.span.start)
}

/// An importer's prefix as a rule writes it (`a.b` of `a.b.c` and of `a.b.{c, d}`), scalafix's `ref`.
fn prefix(s: &str) -> &str {
    let s = s.trim();
    match (s.ends_with('}'), s.find(".{")) {
        (true, Some(i)) => &s[..i],
        _ => s.rfind('.').map_or("", |i| &s[..i]),
    }
}

/// The statements of a source, or of a package clause's body.
fn stats(t: &super::nodes::Tree, n: u32) -> impl Iterator<Item = u32> + '_ {
    let holder = if t.nodes[n as usize].kind == K::Pkg { t.children(n).iter().find(|c| c.0 == super::nodes::F::Body).map_or(n, |c| c.1) } else { n };
    t.children(holder).iter().filter(|c| c.0 == super::nodes::F::Stats).map(|c| c.1)
}

/// scalafix's `extractImports`: the leading imports of the source's statements, or of its package clause's where that
/// is its one statement.
fn leading_imports(d: &DocView) -> Vec<u32> {
    let t = &d.tree;
    let mut n = t.nodes.len() as u32 - 1;
    loop {
        let all: Vec<u32> = stats(t, n).collect();
        match all[..] {
            [only] if t.nodes[only as usize].kind == K::Pkg => n = only,
            _ => return all.into_iter().take_while(|&s| t.nodes[s as usize].kind == K::Import).collect(),
        }
    }
}

/// scalafix's `fallbackToken`, its end: a source whose first statement is a package clause goes into it, and a package
/// clause into its first statement (a package clause with none is the statement); a source whose first statement is
/// none is its start (scalameta's `BOF`). Of the statement, from before its first token or its leading comment's
/// (`begComment`), back over spaces, tabs and comments (`HTrivia`): a line end there gives the token before it, any
/// other token itself.
fn fallback_end(d: &DocView) -> u32 {
    let t = &d.tree;
    let mut n = match stats(t, t.nodes.len() as u32 - 1).next() {
        Some(p) if t.nodes[p as usize].kind == K::Pkg => p,
        _ => return 0,
    };
    while t.nodes[n as usize].kind == K::Pkg {
        match stats(t, n).next() {
            Some(s) => n = s,
            None => break,
        }
    }
    let first = d.tokens.partition_point(|p| p.span.start < t.nodes[n as usize].span.start);
    let from = beg_comment(d, first, t.nodes[n as usize].kind == K::TermBlock).unwrap_or(first);
    let mut i = from;
    loop {
        if i == 0 {
            return 0;
        }
        i -= 1;
        if !matches!(d.tokens[i].kind, Kind::Space | Kind::Tab | Kind::Comment) {
            break;
        }
    }
    if is_line_end(d.tokens[i].kind) {
        if i == 0 { 0 } else { d.tokens[i - 1].span.end }
    } else {
        d.tokens[i].span.end
    }
}

fn is_line_end(k: Kind) -> bool {
    matches!(k, Kind::LF | Kind::CRLF | Kind::CR | Kind::FF)
}

/// scalameta's leading comments of a tree (`begComment`, its parser's `captureComments`), the first one's token: from
/// before the tree's first token back over spaces, tabs, line ends and comments to a blank line or another token, the
/// comments met, less those on that token's line when it can end a statement (`canEndStat`) or is a comma after a line
/// end, or the tree is a block. A token of a quasiquote's or XML's (`CommentEnd`, `Unquote`, `Xml.End`) is not met.
fn beg_comment(d: &DocView, first: usize, block: bool) -> Option<usize> {
    let mut comments: Vec<usize> = Vec::new();
    let (mut pending, mut had_line_end) = (0usize, false);
    let mut i = first;
    while i > 0 {
        i -= 1;
        match d.tokens[i].kind {
            Kind::Comment => {
                comments.insert(0, i);
                pending += 1;
            }
            k if is_line_end(k) => {
                if had_line_end && pending == 0 {
                    break;
                }
                had_line_end = true;
                pending = 0;
            }
            Kind::Space | Kind::Tab => {}
            k => {
                let name = super::source::kind_name(k);
                if !comments.is_empty() && (can_end_stat(d, i, name) || block || had_line_end && name == "Comma") {
                    comments.drain(0..pending);
                }
                break;
            }
        }
    }
    comments.first().copied()
}

/// scalameta's `canEndStat`: an identifier, `given`, a literal, an interpolation's end, `return`, `this`, `type`, a
/// closing bracket of any kind, `_`.
fn can_end_stat(d: &DocView, i: usize, name: &str) -> bool {
    match name {
        "Ident" | "KwGiven" | "KwTrue" | "KwFalse" | "KwNull" | "KwReturn" | "KwThis" | "KwType" | "RightParen" | "RightBracket" | "RightBrace" | "Underscore" => true,
        "Interpolation.Delimiter" => d.text[d.tokens[i].span.start as usize..].starts_with('"') && i > 0 && super::source::kind_name(d.tokens[i - 1].kind) != "Interpolation.Id",
        _ => name.starts_with("Constant."),
    }
}

/// The text with the plan's edits applied.
pub fn apply(text: &str, edits: &[(Span, String, Vec<u32>)]) -> String {
    let mut out = String::with_capacity(text.len() + 64);
    let mut at = 0usize;
    for (span, rep, _) in edits {
        let (s, e) = (span.start as usize, span.end as usize);
        out.push_str(&text[at..s]);
        out.push_str(rep);
        at = e;
    }
    out.push_str(&text[at..]);
    out
}
