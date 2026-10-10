//! The edits that remove a file's unused import selectors, scalac 3.8.4's
//! `CheckUnused.warnings.checkImports` (its `actionable` branch, which 3.8.4 always takes). Per
//! import statement with an unused selector: where no selector is kept the statement is deleted,
//! where one is kept the statement becomes an import of it alone, else each clause without a kept
//! selector is deleted, a clause with one kept selector becomes that selector's import, and each
//! other unused selector is deleted with its comma. dotty attaches the action to the warning of
//! the statement's last unused selector (of each clause, of each selector, where they go apart),
//! the others warning without one, and so does this.
//!
//! The positions are those of dotty's trees, which the parser keeps with each selector
//! (`Import::tree_start`, `qual_end`, `clause_end`): a statement's first clause starts at its
//! `import` keyword and the others at their path (`isPrimaryClause`), a clause ends after its
//! `}`, and the replacement names the path and the selector as written (`textFor`); a clause
//! without a path has no action. `editPosAt` takes the rest of the line where it is blank, and for
//! a deletion the whole line, with a blank line around it where one stands on each side, dotty's
//! whitespace (space, tab, CR) and line breaks (LF, FF, CR, SU) as they are.

use crate::ast::Import;
use crate::source::{Action, Span};

/// One clause of an import statement: dotty's `Import` tree.
struct Clause {
    /// The span's start: the `import` keyword of a statement's first clause, else the path's.
    start: usize,
    /// The span's point, where the path starts.
    point: usize,
    /// The span's end, after the closing brace of a clause in braces.
    end: usize,
    primary: bool,
    /// The path as written.
    qual: (usize, usize),
    /// The clause's selectors, as indices of the file's list, in source order.
    sels: Vec<usize>,
}

/// For each selector of `sels` (a file's, in source order) with `used` false, the action its
/// warning carries, if any.
pub(super) fn import_actions(text: &str, sels: &[&Import], used: &[bool]) -> Vec<Option<Action>> {
    let mut out: Vec<Option<Action>> = vec![None; sels.len()];
    let b = text.as_bytes();
    let mut order: Vec<usize> = (0..sels.len()).collect();
    order.sort_by_key(|&i| (sels[i].span.start, sels[i].selector_span.start));
    let mut clauses: Vec<Option<Clause>> = Vec::new();
    let mut points: Vec<u32> = Vec::new();
    for i in order {
        if points.last() == Some(&sels[i].span.start) {
            if let Some(Some(c)) = clauses.last_mut() {
                c.sels.push(i);
            }
            continue;
        }
        let imp = sels[i];
        points.push(imp.span.start);
        let (point, qual_end) = (imp.span.start as usize, imp.qual_end as usize);
        let clause = Clause { start: imp.tree_start as usize, point, end: imp.clause_end as usize, primary: imp.tree_start < imp.span.start, qual: (point, qual_end), sels: vec![i] };
        clauses.push((qual_end > point).then_some(clause));
    }
    let lines = LineStarts::new(b);
    let mut index = 0;
    while index < clauses.len() {
        // The statement: up to the next clause that starts at an `import` (one whose shape the
        // text does not show ends none).
        let next = (index + 1..clauses.len()).find(|&j| clauses[j].as_ref().is_some_and(|c| c.primary)).unwrap_or(clauses.len());
        if clauses[index..next].iter().all(Option::is_some) {
            let existing: Vec<&Clause> = clauses[index..next].iter().map(|c| c.as_ref().unwrap()).collect();
            statement(text, &lines, &existing, sels, used, &mut out);
        }
        index = next;
    }
    out
}

/// The actions of one statement, its clauses `existing` (dotty's `checkImports` loop body).
fn statement(text: &str, lines: &LineStarts, existing: &[&Clause], sels: &[&Import], used: &[bool], out: &mut [Option<Action>]) {
    let b = text.as_bytes();
    let pairs: Vec<(usize, usize)> = existing.iter().enumerate().flat_map(|(k, c)| c.sels.iter().map(move |&s| (k, s))).collect();
    let (keeping, deleting): (Vec<(usize, usize)>, Vec<(usize, usize)>) = pairs.iter().partition(|&&(_, s)| used[s]);
    let Some(&(_, last)) = deleting.last() else { return };
    let text_for = |(k, s): (usize, usize)| -> String {
        let c = existing[k];
        let sel = sels[s].selector_span;
        format!("{}.{}", &text[c.qual.0..c.qual.1], &text[sel.start as usize..sel.end as usize])
    };
    let whole = (existing[0].start, existing[existing.len() - 1].end);
    if keeping.is_empty() {
        out[last] = Some(action(vec![(edit_pos_at(b, lines, whole, true), String::new())]));
        return;
    }
    if keeping.len() == 1 {
        out[last] = Some(action(vec![(edit_pos_at(b, lines, whole, false), format!("import {}", text_for(keeping[0])))]));
        return;
    }
    let kept = |k: usize| keeping.iter().any(|&(i, _)| i == k);
    let lost: Vec<usize> = (0..existing.len()).filter(|&k| !kept(k)).collect();
    for &k in &lost {
        let c = existing[k];
        let patches = if k == existing.len() - 1 {
            let prev = (0..existing.len()).rev().find(|&i| kept(i)).unwrap();
            let comma = find_from(b, b',', existing[prev].end);
            if comma == usize::MAX {
                continue;
            }
            vec![((comma, existing[prev + 1].start), String::new()), ((c.point, c.end), String::new())]
        } else {
            vec![((c.point, existing[k + 1].start), String::new())]
        };
        out[*c.sels.last().unwrap()] = Some(action(patches));
    }
    let singletons: Vec<usize> = (0..existing.len()).filter(|&k| keeping.iter().filter(|&&(i, _)| i == k).count() == 1).collect();
    let mut seen: Vec<usize> = Vec::new();
    for &(k, s) in &deleting {
        let c = existing[k];
        if singletons.contains(&k) {
            if !seen.contains(&k) {
                seen.push(k);
                let one = *keeping.iter().find(|&&(i, _)| i == k).unwrap();
                out[s] = Some(action(vec![(edit_pos_at(b, lines, (c.point, c.end), false), text_for(one))]));
            }
        } else if !lost.contains(&k) {
            let at = c.sels.iter().position(|&x| x == s).unwrap();
            let span = sels[s].selector_span;
            let patches = if at == c.sels.len() - 1 {
                let prev = c.sels.iter().rposition(|&x| used[x]).unwrap();
                let comma = find_from(b, b',', sels[c.sels[prev]].selector_span.end as usize);
                if comma == usize::MAX {
                    continue;
                }
                vec![((comma, sels[c.sels[prev + 1]].selector_span.start as usize), String::new()), ((span.start as usize, span.end as usize), String::new())]
            } else {
                vec![((span.start as usize, sels[c.sels[at + 1]].selector_span.start as usize), String::new())]
            };
            out[s] = Some(action(patches));
        }
    }
}

fn action(patches: Vec<((usize, usize), String)>) -> Action {
    Action { title: "unused import", description: "remove import", patches: patches.into_iter().map(|((s, e), t)| (Span::new(s as u32, e as u32), t)).collect() }
}

/// The first `c` at or after `from`, dotty's `content.indexOf` (`usize::MAX` for none).
fn find_from(b: &[u8], c: u8, from: usize) -> usize {
    b.get(from..).and_then(|rest| rest.iter().position(|&x| x == c)).map_or(usize::MAX, |i| from + i)
}

/// dotty's `editPosAt`: `(start, end)` with the rest of its line where that is blank; for a
/// deletion of a line otherwise blank, the whole line, and a blank line before it where one
/// follows it too.
fn edit_pos_at(b: &[u8], lines: &LineStarts, (start, end): (usize, usize), for_deletion: bool) -> (usize, usize) {
    let prev = (0..start).rev().find(|&i| !is_whitespace(b[i]));
    let empty_left = prev.is_none_or(|p| is_line_break(b[p]));
    let next = (end..b.len()).find(|&i| !is_whitespace(b[i]));
    let empty_right = next.is_none_or(|n| is_line_break(b[n]));
    let delete_line = empty_left && empty_right && for_deletion;
    let new_end = match next {
        Some(n) if empty_right => n + delete_line as usize,
        _ => end,
    };
    if !delete_line {
        return (start, new_end);
    }
    let mut new_start = prev.map_or(0, |p| p + 1);
    let start_line = lines.line_of(start);
    if start_line > 1 {
        let end_line = lines.line_of(end);
        let preceding = lines.offset(start_line - 1);
        if let Some(succeeding) = lines.offset_opt(end_line + 2) {
            if lines.offset(start_line) - preceding == 1 && succeeding.wrapping_sub(end) == 2 {
                new_start = preceding;
            }
        }
    }
    (new_start, new_end)
}

/// dotty's `Chars.isWhitespace`: no line break but CR.
fn is_whitespace(c: u8) -> bool {
    c == b' ' || c == b'\t' || c == b'\r'
}

/// dotty's `Chars.isLineBreakChar`.
fn is_line_break(c: u8) -> bool {
    matches!(c, b'\n' | 0x0c | b'\r' | 0x1a)
}

/// The starts of the lines as dotty's `SourceFile` counts them: after an LF, an FF, an SU, or a
/// CR not followed by an LF.
struct LineStarts(Vec<usize>);

impl LineStarts {
    fn new(b: &[u8]) -> LineStarts {
        let mut starts = vec![0];
        for (i, &c) in b.iter().enumerate() {
            let brk = if c == b'\r' { i + 1 == b.len() || b[i + 1] != b'\n' } else { is_line_break(c) };
            if brk {
                starts.push(i + 1);
            }
        }
        LineStarts(starts)
    }

    fn line_of(&self, offset: usize) -> usize {
        self.0.partition_point(|&s| s <= offset).saturating_sub(1)
    }

    fn offset(&self, line: usize) -> usize {
        self.0[line.min(self.0.len() - 1)]
    }

    fn offset_opt(&self, line: usize) -> Option<usize> {
        self.0.get(line).copied()
    }
}
