//! Which suites `teq test` runs (docs/TARGETS.md, "The export and the project verbs"): those of the test
//! configuration's analysis that sbt's `definedTests` defines, by name against the fingerprints
//! the frameworks reported to the runner, each given to the frameworks as sbt's `testMap` gives
//! it, then the `Tests.Exclude` names dropped and `testOnly`'s patterns applied.

use super::analysis::Candidate;
use super::runner::{Fingerprint, Kind};
use std::collections::BTreeSet;
use std::fmt;

/// A suite as sbt's discovery defines it: a class or an object and a fingerprint it matches,
/// held by value, as sbt's `TestFramework.matches` compares them.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Defined {
    pub name: String,
    pub kind: Kind,
    pub module: bool,
    pub print: String,
}

/// sbt's `TestDefinition.toString`, which `show <project>/Test/definedTests` prints.
impl fmt::Display for Defined {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self.kind {
            Kind::Subclass => write!(f, "Test {} : subclass({}, {})", self.name, self.module, self.print),
            Kind::Annotated => write!(f, "Test {} : annotation({}, {})", self.name, self.module, self.print),
        }
    }
}

/// sbt's `Tests.discover` over the candidates: one definition per candidate and fingerprint of
/// its kind (an object for a module's) that its own name or a base class names (a subclass
/// fingerprint) or one of its or its methods' annotations names (an annotated one); in order of
/// name, each once.
pub fn defined(candidates: &[Candidate], prints: &[Fingerprint]) -> Vec<Defined> {
    let mut out = BTreeSet::new();
    for c in candidates {
        for p in prints.iter().filter(|p| p.module == c.module) {
            let names = if p.kind == Kind::Subclass { &c.bases } else { &c.annotations };
            if names.contains(&p.name.as_str()) {
                out.insert(Defined { name: c.name.to_string(), kind: p.kind, module: p.module, print: p.name.clone() });
            }
        }
    }
    out.into_iter().collect()
}

/// A suite to run: its name and the framework and fingerprint it runs under.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Suite {
    pub name: String,
    pub framework: usize,
    pub fingerprint: usize,
}

/// sbt's `testMap`: each definition goes to the first framework, in the export's order, that holds
/// a fingerprint equal to its own.
pub fn assign(defined: &[Defined], prints: &[Fingerprint]) -> Vec<Suite> {
    defined
        .iter()
        .filter_map(|d| {
            let p = prints.iter().filter(|p| p.kind == d.kind && p.module == d.module && p.name == d.print).min_by_key(|p| (p.framework, p.index))?;
            Some(Suite { name: d.name.clone(), framework: p.framework, fingerprint: p.index })
        })
        .collect()
}

/// `testOnly`'s selection (sbt's `selectedFilter`): a pattern with `*` matches as a glob whose `*`
/// stands for any text, dots included, one without it the fully qualified name alone; one that
/// opens with `-` excludes what it matches; without an inclusion, every suite is included.
pub struct Patterns {
    include: Vec<String>,
    exclude: Vec<String>,
}

impl Patterns {
    pub fn new(args: &[String]) -> Patterns {
        let (exclude, include): (Vec<&String>, Vec<&String>) = args.iter().partition(|a| a.starts_with('-'));
        Patterns { include: include.into_iter().cloned().collect(), exclude: exclude.into_iter().map(|e| e[1..].to_string()).collect() }
    }

    pub fn is_empty(&self) -> bool {
        self.include.is_empty() && self.exclude.is_empty()
    }

    pub fn matches(&self, name: &str) -> bool {
        (self.include.is_empty() || self.include.iter().any(|p| glob(p, name))) && !self.exclude.iter().any(|p| glob(p, name))
    }
}

/// sbt's `GlobFilter`: the whole name against the pattern, `*` any text.
fn glob(pattern: &str, name: &str) -> bool {
    let parts: Vec<&str> = pattern.split('*').collect();
    let [first, middle @ .., last] = parts.as_slice() else { return pattern == name };
    let Some(mut rest) = name.strip_prefix(first) else { return false };
    for part in middle {
        match rest.find(part) {
            Some(at) => rest = &rest[at + part.len()..],
            None => return false,
        }
    }
    rest.ends_with(last)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn print(framework: usize, index: usize, kind: Kind, module: bool, name: &str) -> Fingerprint {
        Fingerprint { framework, index, kind, module, name: name.to_string() }
    }

    #[test]
    fn discovery_and_assignment_as_sbt_makes_them() {
        let candidates = vec![
            Candidate { name: "p.S", module: false, bases: vec!["p.S", "munit.FunSuite", "munit.Suite"], annotations: vec![] },
            Candidate { name: "p.Z", module: true, bases: vec!["p.Z", "zio.test.ZIOSpecAbstract"], annotations: vec![] },
            Candidate { name: "p.J", module: false, bases: vec!["p.J"], annotations: vec!["org.junit.Test"] },
            Candidate { name: "p.Plain", module: false, bases: vec!["p.Plain", "java.lang.Object"], annotations: vec![] },
            // An object whose class is a suite is no suite: the fingerprint is the class's.
            Candidate { name: "p.ObjectSuite", module: true, bases: vec!["p.ObjectSuite", "munit.Suite"], annotations: vec![] },
        ];
        let prints = vec![
            print(0, 0, Kind::Annotated, false, "org.junit.Test"),
            print(0, 1, Kind::Subclass, false, "junit.framework.TestCase"),
            print(1, 0, Kind::Subclass, false, "munit.Suite"),
            print(2, 0, Kind::Subclass, true, "zio.test.ZIOSpecAbstract"),
            print(3, 0, Kind::Subclass, false, "munit.Suite"),
        ];
        let found = defined(&candidates, &prints);
        let shown: Vec<String> = found.iter().map(Defined::to_string).collect();
        assert_eq!(shown, ["Test p.J : annotation(false, org.junit.Test)", "Test p.S : subclass(false, munit.Suite)", "Test p.Z : subclass(true, zio.test.ZIOSpecAbstract)"]);
        let suites = assign(&found, &prints);
        let given: Vec<(&str, usize, usize)> = suites.iter().map(|s| (s.name.as_str(), s.framework, s.fingerprint)).collect();
        assert_eq!(given, [("p.J", 0, 0), ("p.S", 1, 0), ("p.Z", 2, 0)]);
    }

    #[test]
    fn test_only_patterns() {
        let names = ["meridian.apitest.MunitSuite", "meridian.apitest.ZioSuite", "app.CoreSuite"];
        let selected = |args: &[&str]| {
            let p = Patterns::new(&args.iter().map(|a| a.to_string()).collect::<Vec<_>>());
            names.iter().filter(|n| p.matches(n)).copied().collect::<Vec<_>>()
        };
        assert_eq!(selected(&[]), names);
        assert_eq!(selected(&["*Munit*"]), ["meridian.apitest.MunitSuite"]);
        assert_eq!(selected(&["MunitSuite"]), Vec::<&str>::new());
        assert_eq!(selected(&["app.CoreSuite"]), ["app.CoreSuite"]);
        assert_eq!(selected(&["meridian.*"]), ["meridian.apitest.MunitSuite", "meridian.apitest.ZioSuite"]);
        assert_eq!(selected(&["*Suite", "-*Zio*"]), ["meridian.apitest.MunitSuite", "app.CoreSuite"]);
        assert_eq!(selected(&["-app.CoreSuite"]), ["meridian.apitest.MunitSuite", "meridian.apitest.ZioSuite"]);
        assert_eq!(selected(&["*"]), names);
        assert_eq!(selected(&["m*t*Z*"]), ["meridian.apitest.ZioSuite"]);
        assert!(glob("a*a", "aa") && !glob("a*a", "a") && glob("**", ""));
    }
}
