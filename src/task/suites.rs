//! Which suites `teq test` runs (docs/TARGETS.md, "The export and the project verbs"): those of the test
//! configuration's analysis that sbt's `definedTests` defines, by name against the fingerprints
//! the frameworks reported to the runner, then selected as sbt's `Tests.processOptions` selects them
//! (`plan`: the `Tests.Exclude` names dropped, `testOnly`'s patterns as its ordered filters, one
//! definition per name), and each put in the first of the configuration's groups whose patterns match
//! it (`placed`).

use super::analysis::Candidate;
use super::export::Group;
use super::runner::{Fingerprint, Kind};
use std::collections::{HashMap, HashSet};
use std::fmt;

/// A suite as sbt's discovery defines it: a class or an object and a fingerprint it matches, the
/// framework's at `framework` and `index` (sbt's definition holds the fingerprint itself), its kind,
/// module and name held by value, as sbt's `TestFramework.matches` compares them.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Defined {
    pub name: String,
    pub kind: Kind,
    pub module: bool,
    pub print: String,
    pub framework: usize,
    pub index: usize,
}

impl Defined {
    /// What sbt's `TestDefinition.equals` compares: the name and the fingerprint by value.
    fn value(&self) -> (&str, Kind, bool, &str) {
        (&self.name, self.kind, self.module, &self.print)
    }
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

/// sbt's `Tests.discover` over the candidates (`MA/Tests.scala` 557-595): for each candidate, a class before
/// the object of its name, a definition per fingerprint of its kind (an object for a module's) that its
/// own name or a base class names, the subclass fingerprints in the frameworks' order, then per annotated
/// fingerprint one of its or its methods' annotations names; two frameworks' equal fingerprints each give
/// one. The candidates in their names' order, where sbt's are in its analysis' (a hash map's) order.
pub fn defined(candidates: &[Candidate], prints: &[Fingerprint]) -> Vec<Defined> {
    let mut sorted: Vec<&Candidate> = candidates.iter().collect();
    sorted.sort_by(|a, b| (a.name, a.module).cmp(&(b.name, b.module)));
    sorted.dedup_by(|a, b| (a.name, a.module) == (b.name, b.module));
    let mut out = Vec::new();
    for c in sorted {
        for kind in [Kind::Subclass, Kind::Annotated] {
            for p in prints.iter().filter(|p| p.kind == kind && p.module == c.module) {
                let names = if kind == Kind::Subclass { &c.bases } else { &c.annotations };
                if names.contains(&p.name.as_str()) {
                    out.push(Defined { name: c.name.to_string(), kind, module: p.module, print: p.name.clone(), framework: p.framework, index: p.index });
                }
            }
        }
    }
    out
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
    /// The patterns with `...` as `**` (`expandGlob`), for `ordered` and `takes`.
    include_glob: Vec<String>,
    exclude_glob: Vec<String>,
}

impl Patterns {
    pub fn new(args: &[String]) -> Patterns {
        let (exclude, include): (Vec<&String>, Vec<&String>) = args.iter().partition(|a| a.starts_with('-'));
        let include: Vec<String> = include.into_iter().cloned().collect();
        let exclude: Vec<String> = exclude.into_iter().map(|e| e[1..].to_string()).collect();
        Patterns { include_glob: include.iter().map(|p| expand(p)).collect(), exclude_glob: exclude.iter().map(|p| expand(p)).collect(), include, exclude }
    }

    pub fn is_empty(&self) -> bool {
        self.include.is_empty() && self.exclude.is_empty()
    }

    pub fn matches(&self, name: &str) -> bool {
        (self.include.is_empty() || self.include.iter().any(|p| glob(p, name))) && !self.exclude.iter().any(|p| glob(p, name))
    }

    /// `testOnly`'s ordered filters (sbt's `IncrementalTest.selectedFilter`): one per inclusion, in their order, each
    /// rejecting what an exclusion matches; without an inclusion, one of every suite no exclusion matches; `...` in a
    /// pattern standing for `**` (`expandGlob`).
    pub fn ordered(&self) -> Vec<Box<dyn Fn(&str) -> bool + '_>> {
        let excluded = move |name: &str| self.exclude_glob.iter().any(|p| glob(p, name));
        if self.include_glob.is_empty() {
            return vec![Box::new(move |name: &str| !excluded(name))];
        }
        self.include_glob.iter().map(|p| Box::new(move |name: &str| glob(p, name) && !excluded(name)) as Box<dyn Fn(&str) -> bool>).collect()
    }

    /// Whether a group's patterns take a suite (sbt-teq's `teqTestGroups`, in `testOnly`'s syntax): `matches`, `...`
    /// standing for `**` (`expandGlob`).
    pub fn takes(&self, name: &str) -> bool {
        (self.include_glob.is_empty() || self.include_glob.iter().any(|p| glob(p, name))) && !self.exclude_glob.iter().any(|p| glob(p, name))
    }

    /// Whether a suite is named whole by an inclusion, which marks its definition `explicitlySpecified`, as sbt's
    /// `testOnly` marks one (`Tests.processOptions`' `ExplicitlyRequestedNames`, `isExplicitFqn`: a name without `*`,
    /// `?` or `...`).
    pub fn names(&self, name: &str) -> bool {
        self.include.iter().any(|p| p == name && !p.contains('*') && !p.contains('?') && !p.contains("..."))
    }
}

/// One invocation's plan, which a run runs and `--list` prints: the definitions sbt's `Tests.processOptions` starts
/// from, those `Tests.Exclude` leaves (`definitions`), its ordered filters as the positions of those each accepts
/// (`filters`, which a run's runner reads again as sbt's host reads them), the suites it selects in its order
/// (`selected`, each with its phase) and the phases of them (`phases`, `placed`).
pub struct Plan {
    pub definitions: Vec<Defined>,
    pub filters: Vec<Vec<usize>>,
    pub selected: Vec<(usize, Defined)>,
    pub phases: Vec<Vec<Defined>>,
}

/// sbt's `Tests.processOptions` (`MA/Tests.scala` 254-320) over the definitions discovered: those `excluded` names
/// dropped, the rest `distinct`, then each ordered filter's matches of those, in the filters' order, `distinct` again,
/// then the first definition of each name (`distinctBy`); each in its phase (`placed`).
pub fn plan(defined: Vec<Defined>, excluded: impl Fn(&str) -> bool, filters: &[Box<dyn Fn(&str) -> bool + '_>], groups: &[Group]) -> Plan {
    let definitions: Vec<Defined> = defined.into_iter().filter(|d| !excluded(&d.name)).collect();
    let distinct = |at: &mut dyn Iterator<Item = usize>| {
        let mut seen = HashSet::new();
        at.filter(|&k| seen.insert(definitions[k].value())).collect::<Vec<usize>>()
    };
    let unique = distinct(&mut (0..definitions.len()));
    let accepted: Vec<Vec<usize>> = filters.iter().map(|f| (0..definitions.len()).filter(|&k| f(&definitions[k].name)).collect()).collect();
    let tests = distinct(&mut accepted.iter().flat_map(|a| unique.iter().copied().filter(|k| a.binary_search(k).is_ok())));
    let mut names = HashSet::new();
    let chosen: Vec<Defined> = tests.into_iter().filter(|&k| names.insert(definitions[k].name.as_str())).map(|k| definitions[k].clone()).collect();
    let (selected, phases) = placed(chosen, groups);
    Plan { definitions, filters: accepted, selected, phases }
}

/// One invocation's plan of an export of Mill's, whose runner has no host (`plan` is sbt's host's): the suites
/// discovered, less those `excluded` names, those `filter` accepts, then one definition per name, the one of the lowest
/// kind (a subclass fingerprint's before an annotated one's), then of the earliest framework and fingerprint, across a
/// class and the object of its name, at the name's first place, the names in their order (`ranked`); in the groups'
/// phases, as `plan`'s are, Mill writing none.
pub fn mill_plan(defined: Vec<Defined>, excluded: impl Fn(&str) -> bool, filter: impl Fn(&str) -> bool, groups: &[Group]) -> Plan {
    let chosen = ranked(defined.into_iter().filter(|d| !excluded(&d.name) && filter(&d.name)).collect());
    let (selected, phases) = placed(chosen, groups);
    Plan { definitions: Vec::new(), filters: Vec::new(), selected, phases }
}

/// One definition per name for an export of Mill's: of each name's, the lowest `(kind, framework, index)`, so that a
/// subclass fingerprint's is kept before an annotated one's whether it defines the class or its object, and of equal
/// fingerprints the earliest framework's; at the place of the name's first, found by the name.
fn ranked(defined: Vec<Defined>) -> Vec<Defined> {
    let rank = |d: &Defined| (d.kind, d.framework, d.index);
    let mut out: Vec<Defined> = Vec::new();
    let mut at: HashMap<String, usize> = HashMap::new();
    for d in defined {
        match at.get(&d.name) {
            Some(&i) => {
                if rank(&d) < rank(&out[i]) {
                    out[i] = d;
                }
            }
            None => {
                at.insert(d.name.clone(), out.len());
                out.push(d);
            }
        }
    }
    out
}

/// The suites of one invocation in their groups' order, the plan `teq test` runs and `--list` prints: each suite chosen
/// in the first group whose patterns match it (`Patterns::takes`, so that an empty list takes every suite still left and
/// an exclusion lets a later group take the suite), with its phase, and the phases of them, one per group, an empty one
/// kept, then one of the suites of none; without groups, one phase of them all. Each suite matched once.
fn placed(chosen: Vec<Defined>, groups: &[Group]) -> (Vec<(usize, Defined)>, Vec<Vec<Defined>>) {
    let matchers: Vec<Patterns> = groups.iter().map(|g| Patterns::new(&g.patterns)).collect();
    let mut phases = vec![Vec::new(); groups.len() + 1];
    let mut selected = Vec::with_capacity(chosen.len());
    for d in chosen {
        let at = matchers.iter().position(|m| m.takes(&d.name)).unwrap_or(groups.len());
        phases[at].push(d.clone());
        selected.push((at, d));
    }
    (selected, phases)
}

/// What `--list` prints of a plan: the suites as sbt's `definedTests` shows them, a line each; with groups, each phase
/// under its header, `Group <name>`, an empty group's too, and the suites of none, where there are any, under
/// `Ungrouped`, which no group's header is.
pub fn listing(phases: &[Vec<Defined>], groups: &[Group]) -> String {
    let mut out = String::new();
    for (i, phase) in phases.iter().enumerate() {
        if !groups.is_empty() {
            match groups.get(i) {
                Some(g) => out.push_str(&format!("Group {}\n", g.name)),
                None if !phase.is_empty() => out.push_str("Ungrouped\n"),
                None => {}
            }
        }
        for d in phase {
            out.push_str(&format!("{}\n", d));
        }
    }
    out
}

/// sbt's `IncrementalTest.expandGlob`: `...` as `**`.
fn expand(pattern: &str) -> String {
    pattern.replace("...", "**")
}

/// sbt's `GlobFilter`: the whole name against the pattern, `*` any text.
fn glob(pattern: &str, name: &str) -> bool {
    let (Some(first_star), Some(last_star)) = (pattern.find('*'), pattern.rfind('*')) else { return pattern == name };
    let Some(mut rest) = name.strip_prefix(&pattern[..first_star]) else { return false };
    if first_star < last_star {
        for part in pattern[first_star + 1..last_star].split('*') {
            match rest.find(part) {
                Some(at) => rest = &rest[at + part.len()..],
                None => return false,
            }
        }
    }
    rest.ends_with(&pattern[last_star + 1..])
}

#[cfg(test)]
mod tests {
    use super::*;

    fn print(framework: usize, index: usize, kind: Kind, module: bool, name: &str) -> Fingerprint {
        Fingerprint { framework, index, kind, module, name: name.to_string(), no_args: true }
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
        let shown: Vec<(String, usize, usize)> = found.iter().map(|d| (d.to_string(), d.framework, d.index)).collect();
        let shown: Vec<(&str, usize, usize)> = shown.iter().map(|(s, f, i)| (s.as_str(), *f, *i)).collect();
        // Two frameworks' equal fingerprints each define p.S.
        assert_eq!(
            shown,
            [
                ("Test p.J : annotation(false, org.junit.Test)", 0, 0),
                ("Test p.S : subclass(false, munit.Suite)", 1, 0),
                ("Test p.S : subclass(false, munit.Suite)", 3, 0),
                ("Test p.Z : subclass(true, zio.test.ZIOSpecAbstract)", 2, 0)
            ]
        );
        let suites = assign(&found, &prints);
        let given: Vec<(&str, usize, usize)> = suites.iter().map(|s| (s.name.as_str(), s.framework, s.fingerprint)).collect();
        assert_eq!(given, [("p.J", 0, 0), ("p.S", 1, 0), ("p.S", 1, 0), ("p.Z", 2, 0)]);
    }

    /// Each suite in the first group that matches it, in the groups' order: an exclusion leaving it to a later group,
    /// an empty list taking what is left, empty groups kept, the rest last; without groups one phase.
    #[test]
    fn the_groups_phases() {
        let defined = |name: &str| Defined { name: name.to_string(), kind: Kind::Subclass, module: false, print: "munit.Suite".to_string(), framework: 0, index: 0 };
        let group = |name: &str, patterns: &[&str]| Group { name: name.to_string(), patterns: patterns.iter().map(|p| p.to_string()).collect() };
        let selected: Vec<Defined> = ["a.RepoTest", "a.DoobieUserTest", "a.SkipRepoTest", "a.Plain", "b.Other"].iter().map(|n| defined(n)).collect();
        fn names(phases: &[Vec<Defined>]) -> Vec<Vec<&str>> {
            phases.iter().map(|p| p.iter().map(|d| d.name.as_str()).collect()).collect()
        }
        let groups = [group("database", &["*RepoTest", "*Doobie*Test", "-*Skip*"]), group("none", &["c.*"]), group("rest of a", &["a.*"])];
        let planned = placed(selected.clone(), &groups).1;
        assert_eq!(names(&planned), [vec!["a.RepoTest", "a.DoobieUserTest"], vec![], vec!["a.SkipRepoTest", "a.Plain"], vec!["b.Other"]]);
        assert_eq!(
            listing(&planned, &groups),
            "Group database\nTest a.RepoTest : subclass(false, munit.Suite)\nTest a.DoobieUserTest : subclass(false, munit.Suite)\nGroup none\nGroup rest of a\nTest a.SkipRepoTest : subclass(false, munit.Suite)\nTest a.Plain : subclass(false, munit.Suite)\nUngrouped\nTest b.Other : subclass(false, munit.Suite)\n"
        );
        let all = placed(selected.clone(), &[group("all", &[]), group("late", &["a.*"])]).1;
        assert_eq!(names(&all), [vec!["a.RepoTest", "a.DoobieUserTest", "a.SkipRepoTest", "a.Plain", "b.Other"], vec![], vec![]]);
        assert!(listing(&all, &[group("all", &[]), group("late", &["a.*"])]).ends_with("Group late\n"), "no Ungrouped header where no suite is in none");
        let whole = placed(selected, &[]).1;
        assert_eq!(whole.len(), 1);
        assert_eq!(listing(&whole, &[]).lines().count(), 5);
    }

    /// sbt's `processOptions`: the excluded dropped, the definitions of one value once, the suites in the ordered
    /// filters' order (each pattern's in discovery's), one definition per name, the first; a suite named whole is
    /// explicitly specified, a pattern's or an exclusion's not.
    #[test]
    fn the_selection_as_process_options_makes_it() {
        let candidates = vec![
            Candidate { name: "a.B", module: false, bases: vec!["a.B", "munit.Suite"], annotations: vec![] },
            Candidate { name: "c.C", module: false, bases: vec!["c.C"], annotations: vec!["org.junit.Test", "org.junit.runner.RunWith"] },
            Candidate { name: "a.A", module: false, bases: vec!["a.A", "munit.Suite"], annotations: vec![] },
        ];
        let prints = vec![
            print(0, 0, Kind::Annotated, false, "org.junit.Test"),
            print(0, 1, Kind::Annotated, false, "org.junit.runner.RunWith"),
            print(1, 0, Kind::Subclass, false, "munit.Suite"),
            print(2, 0, Kind::Subclass, false, "munit.Suite"),
        ];
        let found = defined(&candidates, &prints);
        let names = |plan: &Plan| plan.selected.iter().map(|(_, d)| (d.name.clone(), d.framework, d.index)).collect::<Vec<_>>();
        let patterns = Patterns::new(&["*B".to_string(), "a.*".to_string(), "-x.*".to_string()]);
        let plan = plan(found.clone(), |_| false, &patterns.ordered(), &[]);
        assert_eq!(plan.filters, [vec![2, 3], vec![0, 1, 2, 3]]);
        assert_eq!(names(&plan), [("a.B".to_string(), 1, 0), ("a.A".to_string(), 1, 0)]);
        let every = Patterns::new(&[]);
        let all = super::plan(found.clone(), |_| false, &every.ordered(), &[]);
        assert_eq!(names(&all), [("a.A".to_string(), 1, 0), ("a.B".to_string(), 1, 0), ("c.C".to_string(), 0, 0)]);
        let left = super::plan(found, |n| n == "a.A", &every.ordered(), &[]);
        assert_eq!(left.definitions.len(), 4);
        assert_eq!(names(&left), [("a.B".to_string(), 1, 0), ("c.C".to_string(), 0, 0)]);
        let p = Patterns::new(&["p.S".to_string(), "p.*".to_string(), "-p.T".to_string(), "p.U?".to_string()]);
        assert!(p.names("p.S") && !p.names("p.Sx") && !p.names("p.T") && !p.names("p.U?") && !p.names("p.*"));
    }

    /// A class an annotated fingerprint defines whose object a subclass fingerprint defines: sbt's selection keeps the
    /// class's, discovery's first; Mill's the object's, the subclass fingerprint's.
    #[test]
    fn a_class_and_its_object_of_sbt_and_of_mill() {
        let candidates = vec![
            Candidate { name: "p.D", module: true, bases: vec!["p.D", "p.Base"], annotations: vec![] },
            Candidate { name: "p.D", module: false, bases: vec!["p.D"], annotations: vec!["p.Mark"] },
        ];
        let prints = vec![print(0, 0, Kind::Annotated, false, "p.Mark"), print(0, 1, Kind::Subclass, true, "p.Base")];
        let found = defined(&candidates, &prints);
        let shown = |plan: &Plan| plan.selected.iter().map(|(_, d)| d.to_string()).collect::<Vec<_>>();
        let every = Patterns::new(&[]);
        assert_eq!(shown(&plan(found.clone(), |_| false, &every.ordered(), &[])), ["Test p.D : annotation(false, p.Mark)"]);
        assert_eq!(shown(&mill_plan(found, |_| false, |_| true, &[])), ["Test p.D : subclass(true, p.Base)"]);
    }

    /// Each exporter's selection and listing, with patterns and a group, discovery's definitions given, grows with the
    /// suites as the suites do: 20,000 suites in under eight times 5,000's time, the better of two (four for a linear
    /// growth, sixteen for a search of every suite kept before, which Mill's had), whatever the build's optimization or
    /// the machine's load; the names of one length, so that no comparison of two stops at their lengths.
    #[test]
    fn the_selection_grows_with_the_suites() {
        let groups = [Group { name: "ones".to_string(), patterns: vec!["p.S1*".to_string()] }];
        let patterns = Patterns::new(&["p.S2*".to_string(), "p.*".to_string(), "-p.S3*".to_string()]);
        let time = |n: usize, make: &dyn Fn(Vec<Defined>) -> Plan| {
            let candidates: Vec<Candidate> = (0..n).map(|i| Candidate { name: Box::leak(format!("p.S{:05}", i).into_boxed_str()), module: false, bases: vec!["munit.Suite"], annotations: vec![] }).collect();
            let found = defined(&candidates, &[print(0, 0, Kind::Subclass, false, "munit.Suite")]);
            (0..2)
                .map(|_| {
                    let given = found.clone();
                    let start = std::time::Instant::now();
                    let plan = make(given);
                    assert!(listing(&plan.phases, &groups).len() > plan.selected.len());
                    start.elapsed()
                })
                .min()
                .unwrap()
        };
        let sbt = |given| plan(given, |_| false, &patterns.ordered(), &groups);
        let mill = |given| mill_plan(given, |_| false, |n| patterns.matches(n), &groups);
        for (exporter, make) in [("sbt's", &sbt as &dyn Fn(Vec<Defined>) -> Plan), ("Mill's", &mill)] {
            let (small, large) = (time(5_000, make), time(20_000, make));
            assert!(large < small * 8 + std::time::Duration::from_millis(5), "{} export: 20,000 suites in {:?}, 5,000 in {:?}", exporter, large, small);
        }
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
        // `...` a glob's `**` in testOnly's filters and the groups' patterns (sbt's `expandGlob`).
        let p = Patterns::new(&["meridian...Suite".to_string(), "-...Zio...".to_string()]);
        let ordered = p.ordered();
        assert!(ordered.len() == 1 && ordered[0]("meridian.apitest.MunitSuite") && !ordered[0]("meridian.apitest.ZioSuite"));
        assert!(p.takes("meridian.apitest.MunitSuite") && !p.takes("meridian.apitest.ZioSuite") && !p.matches("meridian.apitest.MunitSuite"));
    }
}
