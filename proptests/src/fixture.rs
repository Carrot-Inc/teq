//! The machine over a program written by hand (tests/split/retype, the fixture of the retype's
//! suites), for the controls: whether a known defect is detected and how far its history
//! shrinks, apart from whether the generated programs reach it.
//!
//! The program is taken as its text with sites in it: the string and integer literals outside
//! comments, and the declared result types of its definitions. An edit of a literal leaves the
//! sites where they are, so a site's index means the same in every history.

use crate::driver::{Due, Plan, Texts};
use crate::session;
use hegel::generators as gs;
use hegel::stateful::{Invariant, Rule, StateMachine};
use hegel::TestCase;
use std::cell::RefCell;
use std::collections::BTreeSet;
use std::rc::Rc;

#[derive(Clone, Debug, PartialEq)]
enum Piece {
    Text(String),
    Str { now: String, was: String },
    Int { now: String, was: String },
    Type { name: String, broken: bool },
}

struct Source {
    name: String,
    pieces: Vec<Piece>,
}

pub struct Fixture {
    sources: Vec<Source>,
    /// The literals and the declared types, each as (source, piece).
    literals: Vec<(usize, usize)>,
    types: Vec<(usize, usize)>,
    plan: Rc<RefCell<Plan>>,
    rules: Vec<(&'static str, f64)>,
}

/// Edits of bodies alone.
pub const BODIES: [(&str, f64); 4] = [("literal", 5.0), ("two_files", 2.0), ("nothing", 1.0), ("back", 1.0)];

/// Edits of bodies, and declared types that stop resolving and resolve again.
pub const BODIES_AND_TYPES: [(&str, f64); 6] =
    [("literal", 5.0), ("two_files", 2.0), ("nothing", 1.0), ("back", 1.0), ("break_type", 1.0), ("mend_type", 2.0)];

const WORDS: [&str; 8] = ["a", "bb", "ccc", "one", "two", "three", "dddd", "ee"];

fn index(tc: &TestCase, count: usize) -> usize {
    tc.draw(gs::integers::<usize>().min_value(0).max_value(count - 1))
}

fn is_name(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

/// The pieces of a source text.
fn scan(text: &str) -> Vec<Piece> {
    let mut pieces = Vec::new();
    let mut plain = String::new();
    let mut in_comment = false;
    for line in text.split_inclusive('\n') {
        let trimmed = line.trim_start();
        if in_comment || trimmed.starts_with("/*") || trimmed.starts_with("//") {
            in_comment = (in_comment || trimmed.starts_with("/*")) && !line.contains("*/");
            plain.push_str(line);
            continue;
        }
        let declares = ["def ", "val ", "lazy val ", "inline def "].iter().any(|start| trimmed.starts_with(start));
        let type_at = if declares { declared_type(line) } else { None };
        let chars: Vec<char> = line.chars().collect();
        let mut at = 0;
        while at < chars.len() {
            let c = chars[at];
            let after_name = at > 0 && is_name(chars[at - 1]);
            if let Some((start, end)) = type_at.filter(|(start, _)| *start == at) {
                pieces.push(Piece::Text(std::mem::take(&mut plain)));
                pieces.push(Piece::Type { name: chars[start..end].iter().collect(), broken: false });
                at = end;
            } else if c == '"' {
                let mut end = at + 1;
                while end < chars.len() && chars[end] != '"' {
                    end += if chars[end] == '\\' { 2 } else { 1 };
                }
                let inside: String = chars[at + 1..end.min(chars.len())].iter().collect();
                let interpolated = after_name && !line[..line.char_indices().nth(at).unwrap().0].ends_with("words");
                if interpolated || inside.contains('$') || inside.contains('\\') || end >= chars.len() {
                    plain.extend(&chars[at..(end + 1).min(chars.len())]);
                } else {
                    plain.push('"');
                    pieces.push(Piece::Text(std::mem::take(&mut plain)));
                    pieces.push(Piece::Str { now: inside.clone(), was: inside });
                    plain.push('"');
                }
                at = end + 1;
            } else if c.is_ascii_digit() && !after_name && !(at > 0 && chars[at - 1] == '.') {
                let mut end = at;
                while end < chars.len() && chars[end].is_ascii_digit() {
                    end += 1;
                }
                let digits: String = chars[at..end].iter().collect();
                if end < chars.len() && (is_name(chars[end]) || chars[end] == '.') {
                    plain.push_str(&digits);
                } else {
                    pieces.push(Piece::Text(std::mem::take(&mut plain)));
                    pieces.push(Piece::Int { now: digits.clone(), was: digits });
                }
                at = end;
            } else {
                plain.push(c);
                at += 1;
            }
        }
    }
    pieces.push(Piece::Text(plain));
    pieces
}

/// Where the declared result type of the line's definition stands, in characters: the name
/// before the ` =` that ends the signature.
fn declared_type(line: &str) -> Option<(usize, usize)> {
    let chars: Vec<char> = line.chars().collect();
    let mut depth = 0;
    let mut at = 0;
    while at + 1 < chars.len() {
        match chars[at] {
            '(' | '[' => depth += 1,
            ')' | ']' => depth -= 1,
            ' ' if depth == 0 && chars[at + 1] == '=' && chars.get(at + 2).is_none_or(|c| *c == ' ' || *c == '\n') => {
                let end = at;
                let mut start = end;
                while start > 0 && is_name(chars[start - 1]) {
                    start -= 1;
                }
                let typed = start >= 2 && chars[start - 1] == ' ' && chars[start - 2] == ':';
                return (typed && start < end).then_some((start, end));
            }
            _ => {}
        }
        at += 1;
    }
    None
}

fn text_of(source: &Source) -> String {
    let mut text = String::new();
    for piece in &source.pieces {
        match piece {
            Piece::Text(plain) => text.push_str(plain),
            Piece::Str { now, .. } | Piece::Int { now, .. } => text.push_str(now),
            Piece::Type { broken: true, .. } => text.push_str("Missing"),
            Piece::Type { name, .. } => text.push_str(name),
        }
    }
    text
}

impl Fixture {
    /// The fixture as it stands in the repository.
    pub fn read(kind: session::Kind, test: &str, rules: &[(&'static str, f64)]) -> Fixture {
        let dir = crate::repository().join("tests/split/retype");
        let mut names: Vec<String> = std::fs::read_dir(&dir)
            .unwrap_or_else(|e| panic!("cannot list {}: {e}", dir.display()))
            .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
            .filter(|name| name.ends_with(".scala"))
            .collect();
        names.sort();
        let mut fixture = Fixture {
            sources: Vec::new(),
            literals: Vec::new(),
            types: Vec::new(),
            plan: Rc::new(RefCell::new(Plan::new(kind, test, Texts::new()))),
            rules: rules.to_vec(),
        };
        for name in names {
            let text = std::fs::read_to_string(dir.join(&name)).unwrap();
            let pieces = scan(&text);
            assert_eq!(text_of(&Source { name: name.clone(), pieces: pieces.clone() }), text, "{name} does not render as it reads");
            let source = fixture.sources.len();
            let read_by_macros = ["left.scala", "right.scala", "base.scala"].contains(&name.as_str());
            let sites = match read_by_macros && !crate::known::asked("macro-reads-edited-file") {
                true => &[][..],
                false => &pieces[..],
            };
            for (piece, kind) in sites.iter().enumerate() {
                match kind {
                    Piece::Text(_) => {}
                    Piece::Type { .. } => fixture.types.push((source, piece)),
                    _ => fixture.literals.push((source, piece)),
                }
            }
            fixture.sources.push(Source { name, pieces });
        }
        fixture.plan = Rc::new(RefCell::new(Plan::new(kind, test, fixture.texts())));
        fixture
    }

    pub fn run(self, tc: TestCase, steps: i64) {
        let plan = self.plan.clone();
        hegel::stateful::machine(self).steps(steps).run(tc);
        plan.borrow().run();
    }

    fn texts(&self) -> Texts {
        self.sources.iter().map(|source| (source.name.clone(), text_of(source))).collect()
    }

    fn say(&self, what: String) {
        self.plan.borrow_mut().say(what);
    }

    fn change(&mut self, tc: &TestCase, site: usize) {
        let (source, piece) = self.literals[site];
        let name = self.sources[source].name.clone();
        let said = match &mut self.sources[source].pieces[piece] {
            Piece::Str { now, .. } => {
                let count = 1 + index(tc, 3);
                let chosen: Vec<&str> = (0..count).map(|_| WORDS[index(tc, WORDS.len())]).collect();
                let old = std::mem::replace(now, chosen.join(" "));
                format!("{name}: \"{old}\" becomes \"{now}\"")
            }
            Piece::Int { now, .. } => {
                let old = std::mem::replace(now, index(tc, 13).to_string());
                format!("{name}: {old} becomes {now}")
            }
            _ => unreachable!(),
        };
        self.say(said);
    }
}

fn literal(m: &mut Fixture, tc: TestCase) {
    let site = index(&tc, m.literals.len());
    m.change(&tc, site);
    m.plan.borrow_mut().rule("literal", Due::Changed);
}

fn two_files(m: &mut Fixture, tc: TestCase) {
    let first = index(&tc, m.literals.len());
    let elsewhere: Vec<usize> = (0..m.literals.len()).filter(|site| m.literals[*site].0 != m.literals[first].0).collect();
    let second = elsewhere[index(&tc, elsewhere.len())];
    m.change(&tc, first);
    m.change(&tc, second);
    m.plan.borrow_mut().rule("two_files", Due::Changed);
}

fn nothing(m: &mut Fixture, tc: TestCase) {
    let due = match index(&tc, m.sources.len() + 1) {
        0 => Due::Plain,
        n => Due::Unchanged(m.sources[n - 1].name.clone()),
    };
    m.say(format!("nothing changes, {due:?}"));
    m.plan.borrow_mut().rule("nothing", due);
}

fn back(m: &mut Fixture, tc: TestCase) {
    let changed: Vec<(usize, usize)> = m
        .literals
        .iter()
        .copied()
        .filter(|(source, piece)| matches!(&m.sources[*source].pieces[*piece], Piece::Str { now, was } | Piece::Int { now, was } if now != was))
        .collect();
    if changed.is_empty() {
        m.say("no literal to set back: nothing changes".to_string());
    } else {
        let (source, piece) = changed[index(&tc, changed.len())];
        let name = m.sources[source].name.clone();
        if let Piece::Str { now, was } | Piece::Int { now, was } = &mut m.sources[source].pieces[piece] {
            let said = format!("{name}: {now} goes back to {was}");
            *now = was.clone();
            m.say(said);
        }
    }
    m.plan.borrow_mut().rule("back", Due::Changed);
}

fn set_type(m: &mut Fixture, tc: TestCase, to: bool, rule: &'static str) {
    let sites: Vec<(usize, usize)> = m
        .types
        .iter()
        .copied()
        .filter(|(source, piece)| matches!(&m.sources[*source].pieces[*piece], Piece::Type { broken, .. } if *broken != to))
        .collect();
    if sites.is_empty() {
        m.say("no declared type to change: nothing changes".to_string());
    } else {
        let (source, piece) = sites[index(&tc, sites.len())];
        let name = m.sources[source].name.clone();
        let line = m.sources[source].pieces[..piece].iter().map(|piece| text_of(&Source { name: String::new(), pieces: vec![piece.clone()] })).collect::<String>();
        let line = line.lines().last().unwrap_or("").trim().to_string();
        if let Piece::Type { name: of, broken } = &mut m.sources[source].pieces[piece] {
            *broken = to;
            let said = match to {
                true => format!("{name}: the declared type {of} of `{line}` becomes Missing"),
                false => format!("{name}: the declared type of `{line}` is {of} again"),
            };
            m.say(said);
        }
    }
    m.plan.borrow_mut().rule(rule, Due::Changed);
}

fn break_type(m: &mut Fixture, tc: TestCase) {
    set_type(m, tc, true, "break_type");
}

fn mend_type(m: &mut Fixture, tc: TestCase) {
    set_type(m, tc, false, "mend_type");
}

fn written(m: &mut Fixture, _: TestCase) {
    let macro_files: BTreeSet<String> =
        m.sources.iter().map(|source| source.name.clone()).filter(|name| ["card.scala", "panel.scala", "sides.scala"].contains(&name.as_str())).collect();
    let texts = m.texts();
    let broken = m.sources.iter().any(|source| source.pieces.iter().any(|piece| matches!(piece, Piece::Type { broken: true, .. })));
    m.plan.borrow_mut().step(texts, macro_files, broken);
}

impl StateMachine for Fixture {
    fn rules(&self) -> Vec<Rule<Self>> {
        let apply = |name: &str| -> fn(&mut Fixture, TestCase) {
            match name {
                "literal" => literal,
                "two_files" => two_files,
                "nothing" => nothing,
                "back" => back,
                "break_type" => break_type,
                "mend_type" => mend_type,
                other => panic!("no rule named {other}"),
            }
        };
        self.rules.iter().map(|(name, weight)| Rule::new(name, *weight, apply(name))).collect()
    }

    fn invariants(&self) -> Vec<Invariant<Self>> {
        vec![Invariant::new_always_run("the step is written down", written)]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_the_sites() {
        let pieces = scan("/** a \"comment\" 1 */\nobject Panel:\n  def counted: Int = words\"four five\" + count(\"six\") + xs._1\n  val n = List(1, 22)\n");
        let sites: Vec<&Piece> = pieces.iter().filter(|piece| !matches!(piece, Piece::Text(_))).collect();
        let str = |text: &str| Piece::Str { now: text.to_string(), was: text.to_string() };
        let int = |text: &str| Piece::Int { now: text.to_string(), was: text.to_string() };
        assert_eq!(
            sites,
            [&Piece::Type { name: "Int".to_string(), broken: false }, &str("four five"), &str("six"), &int("1"), &int("22")]
        );
    }
}
