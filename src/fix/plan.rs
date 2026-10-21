//! The patch planner (docs/FIX.md, "Patches"): what the rules returned for a document, filtered by its
//! suppressions, merged, checked and turned into byte edits of its text and its lints.
//!
//! - A rule's own edits of one token merge as scalafix's `PatchInternals.merge` merges them: additions concatenated in
//!   the order made, a removal and an addition a replacement, two removals one.
//! - Across rules, per token: rules whose edits of it are identical count once; additions of several rules to a
//!   token none removes are joined in the configured order of the rules; a removal by one rule and an addition by
//!   another is a conflict (scalafix concatenates there; teq refuses, both rules named).
//! - `removeImportee`s of one statement are planned together through dotty's `checkImports` edits
//!   (`typer::unused_edits::import_actions`): the statement deleted when nothing is kept, a clause with one kept
//!   selector made that selector's import, the others' selectors dropped with their commas; a hiding selector
//!   (`a as _`) whose clause loses its last wildcard goes with it, as `CheckUnused.matchingSelector` counts it used
//!   only beside one. An import statement edited both ways is a conflict.
//! - The edits are applied as dotty's `Rewrites.Patches.apply` applies its patches: identical ones once, by their
//!   start, the untouched bytes copied; a `\n` a rule inserts is written as the file's line ending.

use super::escape::Escapes;
use super::host::{DocView, ImportOp, Lint, Op, Results, Severity};
use super::source::Kind;
use super::nodes::{K, NONE};
use crate::source::Span;
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
    // Atomic groups with an edit where their rule is suppressed go whole (`EscapeHatch.filter`'s `AtomicPatch`, which
    // checks the token edits it applies): each group's added imports at their edit token and its token edits
    // (`atomic_positions`), in that order, up to the first suppressed one; then the import removals' edits as applied.
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
            for (rule, at) in atomic_positions(d, r, g, &written) {
                if !escapes.enabled(&names[rule as usize], at) {
                    dropped.push(g);
                    break;
                }
            }
        }
    }
    // The import removals' edits as they are applied (dotty's `checkImports` edits, which reach from a removed selector
    // to the next one, the comments between them included), each with the atomic groups whose removals made it: a
    // group one of whose edits touches a token its rule is suppressed at goes whole, and the edits are planned again
    // without it, until none does, so that no suppression is deleted by an atomic group.
    let import = loop {
        let removals: Vec<(u32, u32, u32)> = r
            .imports
            .iter()
            .filter_map(|op| match *op {
                ImportOp::Remove { rule, group, importee } if group == 0 || !dropped.contains(&group) => Some((rule, group, importee)),
                _ => None,
            })
            .collect();
        if removals.is_empty() {
            break Ok(Vec::new());
        }
        let planned = match import_edits(d, &removals) {
            Ok(planned) => planned,
            Err(msg) => break Err(msg),
        };
        if escapes.is_empty() {
            break Ok(planned);
        }
        let mut more = false;
        for (span, _, _, groups) in &planned {
            for &g in groups.iter().filter(|&&g| g != 0) {
                if dropped.contains(&g) {
                    continue;
                }
                let Some(&(rule, _, _)) = removals.iter().find(|&&(_, h, _)| h == g) else { continue };
                if touched(d, *span).into_iter().any(|at| !escapes.enabled(&names[rule as usize], at)) {
                    dropped.push(g);
                    more = true;
                }
            }
        }
        if !more {
            break Ok(planned);
        }
    };
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
    let ending = line_ending(&d.text);
    // Per token, per rule, the rule's merged edit: removed, text left of it, text right of it.
    let mut per_token: BTreeMap<u32, BTreeMap<u32, (bool, String, String)>> = BTreeMap::new();
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
        // A rule that removes the token beside another that adds to it.
        let clash = distinct.iter().enumerate().find_map(|(i, (ri, ei))| {
            if !ei.0 {
                return None;
            }
            distinct.iter().enumerate().find(|(j, (_, ej))| *j != i && (!ej.1.is_empty() || !ej.2.is_empty())).map(|(_, (rj, _))| (ri[0], rj[0]))
        });
        if let Some((a, b)) = clash {
            out.refused.push(conflict(d, d.tokens[tok as usize].span, names, a, b));
            continue;
        }
        let span = d.tokens[tok as usize].span;
        let removed = distinct.iter().any(|(_, e)| e.0);
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
    // The import removals, by statement, as planned above.
    match import {
        Ok(import) => {
            for (span, text, rules, _) in import {
                let (span, text) = with_line_end(&d.text, span, &text, ending);
                if let Some(other) = edits.iter().find(|(s, _, _)| s.start < span.end && span.start < s.end) {
                    out.refused.push(conflict(d, span, names, rules[0], other.2[0]));
                    continue;
                }
                edits.push((span, text, rules));
            }
        }
        Err(msg) => out.refused.push(format!("{}: {}", d.path, msg)),
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
    // `Rewrites.Patches.apply`: the patches as collected (the tokens' in their order, then the import removals', then the
    // added imports), each distinct one once, sorted by their start alone (a stable sort), none overlapping the next.
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
/// patch is applied as, `treePatchApply`), each with its rule, but for its import removals (checked as their edits are
/// applied, `plan`): each importer the group adds, unless written or added already, at the edit token it goes right of
/// (`ImportPatchOps`' `extraPatches`); then the group's token edits.
fn atomic_positions(d: &DocView, r: &Results, g: u32, written: &[&str]) -> Vec<(u32, u32)> {
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
    out.extend(r.edits.iter().filter(|e| e.group == g).map(|e| (e.rule, d.tokens[e.token as usize].span.start)));
    out
}

/// The positions of the tokens an edit of `span` touches, in order: each token it overlaps (from the edit's start
/// where a token begins before it), and for an insertion the token at its place.
fn touched(d: &DocView, span: Span) -> Vec<u32> {
    let from = d.tokens.partition_point(|p| p.span.end <= span.start);
    let mut out: Vec<u32> = Vec::new();
    for p in &d.tokens[from..] {
        if p.span.start >= span.end && !(span.start == span.end && p.span.start == span.start) {
            break;
        }
        if p.span.end > p.span.start {
            out.push(p.span.start.max(span.start));
        }
        if span.start == span.end {
            break;
        }
    }
    out
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

/// An edit with the file's line-ending policy: its `\n`s written as the file's ending, and an edit that ends between a
/// CR and its LF (dotty's `editPosAt` takes a CR for blank, `Chars.isWhitespace`) ending after the LF, the line's CRLF
/// written back after its text.
fn with_line_end(text: &str, span: Span, rep: &str, ending: &str) -> (Span, String) {
    let b = text.as_bytes();
    let (s, e) = (span.start as usize, span.end as usize);
    if e > s && b[e - 1] == b'\r' && b.get(e) == Some(&b'\n') {
        return (Span::new(span.start, span.end + 1), format!("{}\r\n", with_ending(rep, ending)));
    }
    (span, with_ending(rep, ending))
}

/// The edits of the statements the removals touch (dotty's `checkImports` edits), with the rules that asked.
fn import_edits(d: &DocView, removals: &[(u32, u32, u32)]) -> Result<Vec<(Span, String, Vec<u32>, Vec<u32>)>, String> {
    let sels: Vec<&crate::ast::Import> = d.imports.iter().collect();
    let mut used = vec![true; sels.len()];
    let mut rules_of: Vec<Vec<u32>> = vec![Vec::new(); sels.len()];
    let mut groups_of: Vec<Vec<u32>> = vec![Vec::new(); sels.len()];
    for &(rule, group, node) in removals {
        let Some(&(_, i)) = d.importee_of.iter().find(|(n, _)| *n == node) else {
            return Err(format!("{} is no importee of the file", &d.text[d.tree.nodes[node as usize].span.start as usize..d.tree.nodes[node as usize].span.end as usize]));
        };
        used[i] = false;
        rules_of[i].push(rule);
        groups_of[i].push(group);
    }
    // A hiding selector goes with its clause's last non-given wildcard (`matchingSelector`'s `masksMatchingMember`).
    for i in 0..sels.len() {
        if used[i] || !matches!(sels[i].sel, crate::ast::ImportSel::Wildcard) {
            continue;
        }
        let clause: Vec<usize> = (0..sels.len()).filter(|&j| sels[j].span.start == sels[i].span.start).collect();
        let wildcard_left = clause.iter().any(|&j| used[j] && matches!(sels[j].sel, crate::ast::ImportSel::Wildcard));
        if wildcard_left {
            continue;
        }
        for &j in &clause {
            if let crate::ast::ImportSel::Name(_, Some(_)) = sels[j].sel {
                let unimport = d.importee_of.iter().any(|&(n, k)| k == j && d.tree.nodes[n as usize].kind == K::ImporteeUnimport);
                if unimport && used[j] {
                    used[j] = false;
                    let r = rules_of[i].clone();
                    rules_of[j] = r;
                    let g = groups_of[i].clone();
                    groups_of[j] = g;
                }
            }
        }
    }
    let actions = crate::typer::unused_edits::import_actions(&d.text, &sels, &used);
    let mut out = Vec::new();
    for (i, a) in actions.into_iter().enumerate() {
        let Some(a) = a else { continue };
        // The action answers to the removed selectors its patches cover (`checkImports`' actions: a statement's or a
        // clause's joint replacement covers each of theirs, a selector's own its own): their rules, and their atomic
        // groups, the provenance `plan` checks, each atomic patch on its own (`EscapeHatch.filter`).
        let covers = |j: usize| {
            let sp = sels[j].selector_span;
            a.patches.iter().any(|(p, _)| p.start <= sp.start && sp.end <= p.end)
        };
        let removed: Vec<usize> = (0..sels.len()).filter(|&j| !used[j] && covers(j)).collect();
        let removed = if removed.is_empty() { vec![i] } else { removed };
        let rules: Vec<u32> = removed.iter().flat_map(|&j| rules_of[j].clone()).collect();
        let mut groups: Vec<u32> = removed.iter().flat_map(|&j| groups_of[j].clone()).collect();
        groups.sort_unstable();
        groups.dedup();
        for (span, text) in a.patches {
            out.push((span, text, if rules.is_empty() { rules_of[i].clone() } else { rules.clone() }, groups.clone()));
        }
    }
    Ok(out)
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
    let top_level = |n: u32| n != NONE && matches!(kind(n), K::Pkg | K::Source);
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

/// The statements of a tree's `stats` field (a source's or a package clause's).
fn stats(t: &super::nodes::Tree, n: u32) -> impl Iterator<Item = u32> + '_ {
    t.children(n).iter().filter(|c| c.0 == super::nodes::F::Stats).map(|c| c.1)
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
