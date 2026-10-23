//! Suppressions (docs/FIX.md, "Suppressions"), scalafix's `EscapeHatch` (scalafix-core's
//! `scalafix/internal/patch/EscapeHatch.scala` at 41cf7983a): `// scalafix:off [rules]` to `// scalafix:on [rules]`,
//! `// scalafix:ok [rules]` after the tree it ends or on its line, and `@SuppressWarnings(Array("[scalafix:]rule"))`
//! on a definition; a rule is matched as a regex found in the lint's full id or the patch's rule
//! (`FilterMatcher`). An anchor or an annotated rule that suppresses nothing is reported
//! (`UnusedScalafixSuppression`).

use super::host::DocView;
use super::nodes::{K, NONE};
use super::source::Kind;
use crate::source::Span;
use std::collections::BTreeSet;

/// Whether `comment` (a comment's text inside its delimiters) is an anchor, with the rules it names (empty for
/// all): `Anchor.OkRgx`, `OnRgx`, `OffRgx`, `\s?scalafix:<anchor>\s?(.*?)(?:;.*)?` matched whole.
pub fn anchor(comment: &str) -> Option<(Anchor, Vec<(String, usize)>)> {
    let lead = if comment.starts_with(|c: char| c.is_whitespace()) { comment.chars().next().unwrap().len_utf8() } else { 0 };
    let rest = comment[lead..].strip_prefix("scalafix:")?;
    let (kind, after) = if let Some(r) = rest.strip_prefix("ok") {
        (Anchor::Ok, r)
    } else if let Some(r) = rest.strip_prefix("on") {
        (Anchor::On, r)
    } else if let Some(r) = rest.strip_prefix("off") {
        (Anchor::Off, r)
    } else {
        return None;
    };
    // `\s?` then the rules up to a `;`, the rest a description.
    let rules_at = lead + "scalafix:".len() + (rest.len() - after.len());
    let (rules_at, rules_text) = match after.chars().next() {
        None => (rules_at, ""),
        Some(c) if c.is_whitespace() => (rules_at + c.len_utf8(), &after[c.len_utf8()..]),
        // `scalafix:onX` is no anchor: `(.*?)` would take `X`, but `\s?` cannot have matched a non-space, and the
        // regex needs the anchor word then `\s?` then anything: `scalafix:onX` matches with rules `X`.
        Some(_) => (rules_at, after),
    };
    let rules_text = rules_text.split(';').next().unwrap_or("");
    // `splitRules`: trimmed, split at commas with the spaces around them; each rule's offset in the comment.
    let trimmed = rules_text.trim();
    let mut rules = Vec::new();
    if !trimmed.is_empty() {
        let mut from = 0;
        for rule in trimmed.split(',').map(str::trim) {
            if rule.is_empty() {
                continue;
            }
            let at = comment[rules_at..].find(rule).map_or(rules_at, |i| rules_at + i);
            let at = comment[from.max(rules_at)..].find(rule).map_or(at, |i| from.max(rules_at) + i);
            from = at + rule.len();
            rules.push((rule.to_string(), at));
        }
    }
    Some((kind, rules))
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Anchor {
    Ok,
    On,
    Off,
}

/// A rule's `FilterMatcher`: everything, or a regex found anywhere in the name.
#[derive(Clone)]
enum Matcher {
    All,
    Rule(std::rc::Rc<crate::interp::regex::Regex>),
}

impl std::fmt::Debug for Matcher {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(if matches!(self, Matcher::All) { "All" } else { "Rule" })
    }
}

impl Matcher {
    /// A rule as written, a Java regex as `FilterMatcher(rule)` compiles it; one that is no regex matches its
    /// text alone.
    fn rule(r: &str) -> Matcher {
        let re = crate::interp::regex::Regex::compile(r).or_else(|_| crate::interp::regex::Regex::compile(&quote(r)));
        match re {
            Ok(re) => Matcher::Rule(std::rc::Rc::new(re)),
            Err(_) => Matcher::All,
        }
    }

    fn matches(&self, name: &str) -> bool {
        match self {
            Matcher::All => true,
            Matcher::Rule(re) => found(re, name),
        }
    }
}

/// `findFirstIn` of `re` in `text`.
pub fn found(re: &crate::interp::regex::Regex, text: &str) -> bool {
    let (chars, _) = crate::interp::regex::unit_chars(text);
    matches!(re.find(&chars, 0, false, false), Ok(Some(_)))
}

fn quote(s: &str) -> String {
    format!("\\Q{}\\E", s.replace("\\E", "\\E\\\\E\\Q"))
}

#[derive(Clone, Debug)]
struct Filter {
    matcher: Matcher,
    /// Where the rule is written, or the anchor where it names none.
    cause: Span,
    start: u32,
    end: Option<u32>,
    /// An annotation's filter is reported unused only where its rule is written `scalafix:`.
    reportable: bool,
}

/// The suppressions of one document.
pub struct Escapes {
    annotated: Vec<Filter>,
    disabling: Vec<Filter>,
    enabling: Vec<Filter>,
    /// Anchors found unused as they were read (`OnOffTracker`, filters of one offset beside a wider one).
    unused: Vec<Span>,
    used: BTreeSet<(bool, usize)>,
}

impl Escapes {
    pub fn of(d: &DocView) -> Escapes {
        let mut e = Escapes { annotated: Vec::new(), disabling: Vec::new(), enabling: Vec::new(), unused: Vec::new(), used: BTreeSet::new() };
        if d.text.contains("SuppressWarnings") {
            e.annotated = annotated(d);
        }
        if d.text.contains("scalafix:") {
            e.anchored(d);
        }
        e
    }

    pub fn is_empty(&self) -> bool {
        self.annotated.is_empty() && self.disabling.is_empty() && self.unused.is_empty()
    }

    /// Whether `name` (a lint's full id, a patch's rule) is enabled at `at`, the suppression that disabled it marked
    /// used: annotations first, then the anchors (`EscapeHatch.isDisabledByEscape`).
    pub fn enabled(&mut self, name: &str, at: u32) -> bool {
        if let Some(i) = self.annotated.iter().position(|f| f.start <= at && f.end.is_some_and(|e| e >= at) && f.matcher.matches(name)) {
            self.used.insert((true, i));
            return false;
        }
        let mut culprit: Option<usize> = None;
        let mut ids: Vec<usize> = (0..self.disabling.len()).filter(|&i| self.disabling[i].start <= at).collect();
        ids.sort_by_key(|&i| self.disabling[i].start);
        for i in ids {
            let f = &self.disabling[i];
            if !f.matcher.matches(name) {
                continue;
            }
            match f.end {
                Some(end) if at < end => {
                    self.used.insert((false, i));
                    return false;
                }
                Some(_) => {}
                None => {
                    let start = f.start;
                    match self.enabling.iter().find(|e| e.start >= start && e.start < at && e.matcher.matches(name)) {
                        None => {
                            self.used.insert((false, i));
                            return false;
                        }
                        Some(_) => culprit = Some(i),
                    }
                }
            }
        }
        let _ = culprit;
        true
    }

    /// The suppressions that suppressed nothing: the anchors found unused as they were read, every anchor filter
    /// never used, and every annotated rule written `scalafix:` never used.
    pub fn unused(&self) -> Vec<Span> {
        let mut out: Vec<Span> = self.unused.clone();
        for (i, f) in self.disabling.iter().enumerate() {
            if !self.used.contains(&(false, i)) {
                out.push(f.cause);
            }
        }
        for (i, f) in self.annotated.iter().enumerate() {
            if f.reportable && !self.used.contains(&(true, i)) {
                out.push(f.cause);
            }
        }
        out.sort_by_key(|s| (s.start, s.end));
        out.dedup();
        out
    }

    fn anchored(&mut self, d: &DocView) {
        let mut disable: Vec<(u32, Vec<Filter>)> = Vec::new();
        let mut visited: BTreeSet<u32> = BTreeSet::new();
        let comments: Vec<(Span, String)> = d
            .tokens
            .iter()
            .filter(|p| p.kind == Kind::Comment)
            .map(|p| (p.span, comment_value(&d.text[p.span.start as usize..p.span.end as usize]).to_string()))
            .collect();
        let line_start = |at: u32| d.text[..at as usize].rfind('\n').map_or(0, |i| i as u32 + 1);
        if d.text.contains("scalafix:ok") {
            // The trees an ok anchor trails, in pre-order (`tree.foreach` of `t.endComment`, the comments the parser
            // gives the tree): from the start of the tree's first line to its end.
            let mut order: Vec<u32> = (0..d.tree.nodes.len() as u32).collect();
            preorder(d, &mut order);
            for n in order {
                let node = d.tree.nodes[n as usize];
                let Some(ec) = node.end_comment else { continue };
                for &(span, ref value) in comments.iter().filter(|(s, _)| s.start >= ec.start && s.end <= ec.end) {
                    if let Some((Anchor::Ok, rules)) = anchor(value) {
                        if visited.insert(span.start) {
                            let start = line_start(node.span.start);
                            disable.push((start, filters(&rules, span, value_offset(&d.text, span), start, Some(node.span.end))));
                        }
                    }
                }
            }
        }
        let mut tracker = Tracker::default();
        for (span, value) in &comments {
            let Some((kind, rules)) = anchor(value) else { continue };
            let base = value_offset(&d.text, *span);
            match kind {
                Anchor::Off => {
                    let fs = filters(&rules, *span, base, span.start, None);
                    tracker.off(&rules, *span, base, &mut self.unused);
                    disable.push((span.start, fs));
                }
                Anchor::On => {
                    let fs = filters(&rules, *span, base, span.start, None);
                    tracker.on(&rules, *span, base, &mut self.unused);
                    self.enabling.extend(fs);
                }
                Anchor::Ok => {
                    if !visited.contains(&span.start) {
                        let start = line_start(span.start);
                        disable.push((start, filters(&rules, *span, base, start, Some(span.end))));
                    }
                }
            }
        }
        // For filters starting at one offset, the one of the wider range stays and the others are unused.
        disable.sort_by_key(|(start, fs)| (*start, std::cmp::Reverse(fs.first().and_then(|f| f.end).unwrap_or(u32::MAX))));
        let mut i = 0;
        while i < disable.len() {
            let start = disable[i].0;
            self.disabling.extend(disable[i].1.iter().cloned());
            let mut j = i + 1;
            while j < disable.len() && disable[j].0 == start {
                self.unused.extend(disable[j].1.iter().map(|f| f.cause));
                j += 1;
            }
            i = j;
        }
        self.enabling.sort_by_key(|f| f.start);
    }
}

/// A comment's text inside its delimiters, scalameta's `Comment.value`.
fn comment_value(c: &str) -> &str {
    if let Some(r) = c.strip_prefix("//") {
        r
    } else {
        c.strip_prefix("/*").and_then(|r| r.strip_suffix("*/")).unwrap_or(c)
    }
}

/// Where a comment's value starts in the text.
fn value_offset(text: &str, span: Span) -> u32 {
    let _ = text;
    span.start + 2
}

fn filters(rules: &[(String, usize)], anchor: Span, base: u32, start: u32, end: Option<u32>) -> Vec<Filter> {
    if rules.is_empty() {
        return vec![Filter { matcher: Matcher::All, cause: anchor, start, end, reportable: true }];
    }
    rules
        .iter()
        .map(|(r, at)| {
            let s = base + *at as u32;
            Filter { matcher: Matcher::rule(r), cause: Span::new(s, s + r.len() as u32), start, end, reportable: true }
        })
        .collect()
}

/// The nodes in pre-order from the root, outermost first.
fn preorder(d: &DocView, out: &mut Vec<u32>) {
    out.clear();
    let root = d.tree.nodes.len() as u32 - 1;
    let mut stack = vec![root];
    while let Some(n) = stack.pop() {
        out.push(n);
        for (_, c) in d.tree.written_children(n).rev() {
            stack.push(c);
        }
    }
}

/// `OnOffTracker`: which rules are on or off as the anchors are read top to bottom, an anchor that changes
/// nothing unused.
#[derive(Default)]
struct Tracker {
    /// The rules turned off (`currentlyOff`); `all_off` where a wildcard turned every rule off.
    off: BTreeSet<String>,
    all_off: bool,
}

impl Tracker {
    fn off(&mut self, rules: &[(String, usize)], anchor: Span, base: u32, unused: &mut Vec<Span>) {
        if rules.is_empty() {
            // `(SomeRules(src), AllRules) if src.isEmpty`: every rule already off.
            if self.all_off && self.off.is_empty() {
                unused.push(anchor);
            }
            self.all_off = true;
            self.off.clear();
            return;
        }
        if self.all_off {
            // Source AllRules off: each named rule leaves the set of those still on.
            for (r, at) in rules {
                if !self.off.remove(r) {
                    unused.push(Span::new(base + *at as u32, base + (*at + r.len()) as u32));
                }
            }
        } else {
            for (r, at) in rules {
                if !self.off.insert(r.clone()) {
                    unused.push(Span::new(base + *at as u32, base + (*at + r.len()) as u32));
                }
            }
        }
    }

    fn on(&mut self, rules: &[(String, usize)], anchor: Span, base: u32, unused: &mut Vec<Span>) {
        if rules.is_empty() {
            if !self.all_off && self.off.is_empty() {
                unused.push(anchor);
            }
            self.all_off = false;
            self.off.clear();
            return;
        }
        if self.all_off {
            for (r, at) in rules {
                if !self.off.insert(r.clone()) {
                    unused.push(Span::new(base + *at as u32, base + (*at + r.len()) as u32));
                }
            }
        } else {
            for (r, at) in rules {
                if !self.off.remove(r) {
                    unused.push(Span::new(base + *at as u32, base + (*at + r.len()) as u32));
                }
            }
        }
    }
}

/// The values of the argument clauses of an application or an `Init`, in order.
fn clause_values(t: &super::nodes::Tree, n: u32) -> Vec<u32> {
    t.children(n)
        .iter()
        .filter(|&&(_, c)| t.nodes[c as usize].kind == K::TermArgClause)
        .flat_map(|&(_, c)| t.children(c).iter().filter(|e| e.0 == super::nodes::F::Values).map(|e| e.1))
        .collect()
}

/// `AnnotatedEscapes`: each definition's `@SuppressWarnings(Array(rules))` over the definition's range.
fn annotated(d: &DocView) -> Vec<Filter> {
    let t = &d.tree;
    let mut out = Vec::new();
    for (n, node) in t.nodes.iter().enumerate() {
        if node.kind != K::ModAnnot {
            continue;
        }
        let owner = node.parent;
        if owner == NONE {
            continue;
        }
        let Some(init) = t.children(n as u32).iter().find(|c| t.nodes[c.1 as usize].kind == K::Init).map(|c| c.1) else { continue };
        let kids = t.children(init);
        let Some(&(_, tpe)) = kids.first() else { continue };
        let named = match t.nodes[tpe as usize].kind {
            K::TypeName => t.value(tpe) == Some("SuppressWarnings"),
            K::TypeSelect => t.children(tpe).iter().any(|&(f, c)| f == super::nodes::F::Name && t.value(c) == Some("SuppressWarnings")),
            _ => false,
        };
        if !named {
            continue;
        }
        // The one argument `Array(...)`, its string literals the rules.
        let args: Vec<u32> = clause_values(t, init);
        let [array] = args[..] else { continue };
        if t.nodes[array as usize].kind != K::TermApply {
            continue;
        }
        let fun_ok = t.children(array).iter().any(|&(f, c)| f == super::nodes::F::Fun && t.nodes[c as usize].kind == K::TermName && t.value(c) == Some("Array"));
        if !fun_ok {
            continue;
        }
        let owner_span = t.nodes[owner as usize].span;
        let mut all = Vec::new();
        let mut one = Vec::new();
        for lit in clause_values(t, array) {
            if t.nodes[lit as usize].kind != K::LitString {
                continue;
            }
            let rule = t.value(lit).unwrap_or("").to_string();
            let lspan = t.nodes[lit as usize].span;
            let text = &d.text[lspan.start as usize..lspan.end as usize];
            let lo = lspan.start + text.find(&rule).unwrap_or(0) as u32;
            let cause = Span::new(lo, lo + rule.len() as u32);
            if rule == "all" {
                all.push(Filter { matcher: Matcher::All, cause, start: owner_span.start, end: Some(owner_span.end), reportable: false });
            } else {
                let reportable = rule.starts_with("scalafix:");
                let name = rule.strip_prefix("scalafix:").unwrap_or(&rule).to_string();
                one.push(Filter { matcher: Matcher::rule(&name), cause, start: owner_span.start, end: Some(owner_span.end), reportable });
            }
        }
        out.extend(all);
        out.extend(one);
    }
    out.sort_by_key(|f| f.start);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn anchors() {
        assert_eq!(anchor(" scalafix:ok").map(|a| a.0), Some(Anchor::Ok));
        let (k, rules) = anchor(" scalafix:off NoDummy, NoNull; because").unwrap();
        assert_eq!(k, Anchor::Off);
        assert_eq!(rules.iter().map(|r| r.0.as_str()).collect::<Vec<_>>(), ["NoDummy", "NoNull"]);
        assert_eq!(rules[0].1, " scalafix:off ".len());
        assert!(anchor(" not an anchor").is_none());
        assert!(Matcher::rule("NoDummy").matches("NoDummy"));
        assert!(Matcher::rule("DisableSyntax").matches("DisableSyntax.var"));
        assert!(Matcher::rule(".*").matches("x"));
        assert!(!Matcher::rule("NoNull").matches("NoDummy"));
    }
}
