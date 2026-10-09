//! The analysis of a resident's closure, merged across its answers: a first or full build answers
//! every file of the closure, an incremental one the files it typed again and the files removed,
//! a build that changed nothing no analysis at all (`src/watch.rs`). Kept per resident is what
//! the test verb reads (docs/TARGETS.md, "The export and the project verbs"): per source file, its classes as zinc's
//! discovery reads them with the class files each stands behind, its local classes' class files,
//! and the classes of the build each of its classes depends on, the version 3 API graph's
//! `deps`; the graph's nodes and used names are not kept. Names are interned.

use super::resident::{items, top_level_fields};
use crate::lsp::json::Json;
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::sync::Arc;

/// An interned name.
type Id = u32;

#[derive(Default)]
struct Names {
    ids: HashMap<Arc<str>, Id>,
    names: Vec<Arc<str>>,
}

impl Names {
    fn id(&mut self, name: &str) -> Id {
        if let Some(&id) = self.ids.get(name) {
            return id;
        }
        let name: Arc<str> = Arc::from(name);
        let id = self.names.len() as Id;
        self.names.push(name.clone());
        self.ids.insert(name, id);
        id
    }

    fn name(&self, id: Id) -> &str {
        &self.names[id as usize]
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Kind {
    Class,
    Trait,
    Object,
}

struct Class {
    name: Id,
    kind: Kind,
    top: bool,
    public: bool,
    abstract_: bool,
    /// Whether the `java` launcher runs it: an object with a `main(args: Array[String])` of its
    /// own or inherited, or the class of a `@main` method (`src/jvm/analysis.rs`).
    main: bool,
    /// The linearisation, the class's own name not among it.
    bases: Vec<Id>,
    /// The class's annotations and those of its public methods.
    annotations: Vec<Id>,
    /// The class files it stands behind, relative to the class directory.
    files: Vec<Box<str>>,
}

#[derive(Default)]
struct File {
    classes: Vec<Class>,
    local: Vec<Box<str>>,
    /// Per class of the file, the classes of the build it depends on.
    deps: Vec<(Id, Vec<Id>)>,
}

/// A class of the test sources as discovery matches it against the frameworks' fingerprints.
#[derive(Debug, PartialEq, Eq)]
pub struct Candidate<'a> {
    pub name: &'a str,
    pub module: bool,
    /// Its own name and its linearisation's.
    pub bases: Vec<&'a str>,
    pub annotations: Vec<&'a str>,
}

#[derive(Default)]
pub struct View {
    names: Names,
    files: BTreeMap<Box<str>, File>,
    /// Why the dependencies of some files are unknown: an answer carried their analysis without
    /// the API graph. A full build's graph clears it.
    pub deps_missing: Option<String>,
    /// Why the view is empty: an answer whose analysis did not read.
    pub unreadable: Option<String>,
}

/// What an answer carries for the view, its raw fields.
pub struct Fields<'a> {
    pub analysis: Option<&'a str>,
    pub api: Option<&'a str>,
    pub api_errors: Option<&'a str>,
    pub incremental: bool,
    pub removed: &'a [String],
}

impl View {
    pub fn unreadable(why: String) -> View {
        View { unreadable: Some(why), ..View::default() }
    }

    /// Folds an answer in: nothing when it carries no analysis, its files replacing theirs and
    /// its removed files dropped when it is incremental, the whole view replaced otherwise.
    pub fn merge(&mut self, answer: &Fields) -> Result<(), String> {
        let Some(analysis) = answer.analysis else { return Ok(()) };
        if !answer.incremental {
            *self = View::default();
        }
        for path in answer.removed {
            self.files.remove(normal(path).as_str());
        }
        for item in items(analysis) {
            let json = Json::parse(item).map_err(|e| format!("the analysis of a file does not read: {}", e))?;
            let path = json.get("file").and_then(Json::str).ok_or("the analysis names a file without its path")?;
            let mut file = File { local: strings(json.get("local")).map(Box::from).collect(), ..File::default() };
            for c in json.get("classes").map_or(&[][..], Json::arr) {
                let flag = |k: &str| c.get(k).and_then(Json::bool) == Some(true);
                let kind = match c.get("kind").and_then(Json::str) {
                    Some("trait") => Kind::Trait,
                    Some("object") => Kind::Object,
                    _ => Kind::Class,
                };
                let name = self.names.id(c.get("name").and_then(Json::str).unwrap_or(""));
                let bases = strings(c.get("bases")).map(|b| self.names.id(b)).collect();
                let annotations = strings(c.get("annotations")).chain(strings(c.get("methodAnnotations"))).map(|a| self.names.id(a)).collect();
                let files = strings(c.get("files")).map(Box::from).collect();
                file.classes.push(Class { name, kind, top: flag("top"), public: flag("public"), abstract_: flag("abstract") || kind == Kind::Trait, main: flag("main"), bases, annotations, files });
            }
            self.files.insert(normal(path).into(), file);
        }
        match (answer.api, answer.api_errors) {
            (Some(api), None) => {
                self.merge_deps(api)?;
                if !answer.incremental {
                    self.deps_missing = None;
                }
            }
            (_, errors) => {
                let why = errors.and_then(|e| Json::parse(e).ok()).and_then(|e| e.arr().first().and_then(Json::str).map(str::to_string));
                self.deps_missing = Some(why.unwrap_or_else(|| "the answer carried no API graph".to_string()));
            }
        }
        Ok(())
    }

    /// The `deps` of each file of the API graph: per class, the classes of the build it names.
    fn merge_deps(&mut self, api: &str) -> Result<(), String> {
        let fields = top_level_fields(api);
        let Some((_, files)) = fields.iter().find(|(k, _)| k == "files") else { return Ok(()) };
        for item in items(files) {
            let fields = top_level_fields(item);
            let raw = |key: &str| fields.iter().find(|(k, _)| k == key).map(|(_, v)| *v);
            let Some(Json::Str(path)) = raw("file").and_then(|f| Json::parse(f).ok()) else { return Err("the API graph names a file without its path".to_string()) };
            let mut deps = Vec::new();
            for (class, record) in raw("deps").map(top_level_fields).unwrap_or_default() {
                let on = top_level_fields(record).into_iter().find(|(k, _)| k == "classes").and_then(|(_, v)| Json::parse(v).ok());
                let on: Vec<Id> = on.as_ref().map_or(&[][..], Json::arr).iter().filter_map(|pair| pair.arr().first().and_then(Json::str)).map(|n| self.names.id(n)).collect();
                deps.push((self.names.id(&class), on));
            }
            if let Some(file) = self.files.get_mut(normal(&path).as_str()) {
                file.deps = deps;
            }
        }
        Ok(())
    }

    /// The top-level, public, concrete classes and objects of the files under the roots, as
    /// sbt's discovery takes them from a test configuration's analysis.
    pub fn candidates(&self, roots: &[String]) -> Vec<Candidate<'_>> {
        let mut out = Vec::new();
        for (path, file) in &self.files {
            if !under(path, roots) {
                continue;
            }
            for c in file.classes.iter().filter(|c| c.top && c.public && !c.abstract_) {
                let name = self.names.name(c.name);
                out.push(Candidate {
                    name,
                    module: c.kind == Kind::Object,
                    bases: std::iter::once(name).chain(c.bases.iter().map(|&b| self.names.name(b))).collect(),
                    annotations: c.annotations.iter().map(|&a| self.names.name(a)).collect(),
                });
            }
        }
        out
    }

    /// The main classes among the files under the roots, sorted: what sbt's
    /// `discoveredMainClasses` finds in a configuration's products (a package-private object
    /// among them, unlike `candidates`), read from the `main` the emitter marked each class
    /// with, no class file opened.
    pub fn main_classes_under(&self, roots: &[String]) -> Vec<&str> {
        let mut out: Vec<&str> = self.files.iter().filter(|(path, _)| under(path, roots)).flat_map(|(_, f)| f.classes.iter().filter(|c| c.main).map(|c| self.names.name(c.name))).collect();
        out.sort_unstable();
        out.dedup();
        out
    }

    /// The classes behind the class files, and every class whose dependencies reach one of them,
    /// transitively; a local class's file stands for every class of its source file.
    pub fn affected(&self, class_files: &BTreeSet<String>) -> BTreeSet<String> {
        let mut reached: BTreeSet<Id> = BTreeSet::new();
        for file in self.files.values() {
            let local = file.local.iter().any(|l| class_files.contains(&**l));
            for c in &file.classes {
                if local || c.files.iter().any(|f| class_files.contains(&**f)) {
                    reached.insert(c.name);
                }
            }
        }
        let mut dependents: HashMap<Id, Vec<Id>> = HashMap::new();
        for file in self.files.values() {
            for (class, on) in &file.deps {
                for &o in on {
                    dependents.entry(o).or_default().push(*class);
                }
            }
        }
        let mut work: Vec<Id> = reached.iter().copied().collect();
        while let Some(id) = work.pop() {
            for &d in dependents.get(&id).map_or(&[][..], Vec::as_slice) {
                if reached.insert(d) {
                    work.push(d);
                }
            }
        }
        reached.into_iter().map(|id| self.names.name(id).to_string()).collect()
    }

    /// The class files of the source files under the roots, their local classes' among them.
    pub fn class_files_under(&self, roots: &[String]) -> Vec<&str> {
        let mut out = Vec::new();
        for (_, file) in self.files.iter().filter(|(path, _)| under(path, roots)) {
            out.extend(file.classes.iter().flat_map(|c| c.files.iter()).map(|f| &**f));
            out.extend(file.local.iter().map(|f| &**f));
        }
        out
    }

    /// Every class file a source file stands behind.
    pub fn class_files(&self) -> BTreeSet<&str> {
        self.files.values().flat_map(|f| f.classes.iter().flat_map(|c| c.files.iter()).chain(f.local.iter())).map(|f| &**f).collect()
    }

    pub fn file_count(&self) -> usize {
        self.files.len()
    }

    pub fn class_count(&self) -> usize {
        self.files.values().map(|f| f.classes.len()).sum()
    }

    /// The bytes the view holds on the heap, by its collections' capacities, the maps' entries
    /// counted at their payload.
    pub fn heap_bytes(&self) -> usize {
        use std::mem::size_of;
        let boxed = |s: &Box<str>| s.len() + size_of::<Box<str>>();
        let names = self.names.names.iter().map(|n| n.len() + 16).sum::<usize>()
            + self.names.names.capacity() * size_of::<Arc<str>>()
            + self.names.ids.capacity() * (size_of::<Arc<str>>() + size_of::<Id>() + 1);
        let files: usize = self
            .files
            .iter()
            .map(|(path, f)| {
                boxed(path)
                    + size_of::<File>()
                    + f.local.iter().map(boxed).sum::<usize>()
                    + f.classes.capacity() * size_of::<Class>()
                    + f.classes.iter().map(|c| (c.bases.capacity() + c.annotations.capacity()) * size_of::<Id>() + c.files.iter().map(boxed).sum::<usize>()).sum::<usize>()
                    + f.deps.capacity() * size_of::<(Id, Vec<Id>)>()
                    + f.deps.iter().map(|(_, on)| on.capacity() * size_of::<Id>()).sum::<usize>()
            })
            .sum();
        names + files
    }
}

/// Whether a source file's path lies under one of the roots.
fn under(path: &str, roots: &[String]) -> bool {
    roots.iter().any(|r| path.strip_prefix(r.trim_end_matches('/')).is_some_and(|rest| rest.starts_with('/')))
}

fn strings(json: Option<&Json>) -> impl Iterator<Item = &str> {
    json.map_or(&[][..], Json::arr).iter().filter_map(Json::str)
}

/// A path as the export writes them, `/`-separated.
fn normal(path: &str) -> String {
    path.replace('\\', "/")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn class(name: &str, kind: &str, flags: (bool, bool, bool), bases: &[&str], files: &[&str]) -> String {
        class_with_main(name, kind, flags, bases, files, false)
    }

    fn class_with_main(name: &str, kind: &str, flags: (bool, bool, bool), bases: &[&str], files: &[&str], main: bool) -> String {
        let list = |xs: &[&str]| xs.iter().map(|x| format!("\"{}\"", x)).collect::<Vec<_>>().join(",");
        format!(
            r#"{{"name":"{}","kind":"{}","top":{},"public":{},"abstract":{},"final":false,"bases":[{}],"annotations":[],"methodAnnotations":[],"main":{},"files":[{}]}}"#,
            name,
            kind,
            flags.0,
            flags.1,
            flags.2,
            list(bases),
            main,
            list(files)
        )
    }

    fn analysis(files: &[(&str, Vec<String>, &[&str])]) -> String {
        let entries: Vec<String> = files
            .iter()
            .map(|(path, classes, local)| format!(r#"{{"file":"{}","classes":[{}],"local":[{}]}}"#, path, classes.join(","), local.iter().map(|l| format!("\"{}\"", l)).collect::<Vec<_>>().join(",")))
            .collect();
        format!("[{}]", entries.join(","))
    }

    fn api(files: &[(&str, &[(&str, &[&str])])]) -> String {
        let entries: Vec<String> = files
            .iter()
            .map(|(path, deps)| {
                let deps: Vec<String> = deps
                    .iter()
                    .map(|(c, on)| format!(r#""{}":{{"names":["x"],"patmat":[],"classes":[{}],"binaries":[[0,"scala.Int","DependencyByMemberRef"]]}}"#, c, on.iter().map(|o| format!(r#"["{}","DependencyByMemberRef"]"#, o)).collect::<Vec<_>>().join(",")))
                    .collect();
                format!(r#"{{"file":"{}","classes":[0],"products":[],"local":[],"deps":{{{}}}}}"#, path, deps.join(","))
            })
            .collect();
        format!(r#"{{"nodes":[["Public"]],"files":[{}],"entries":["a.jar"]}}"#, entries.join(","))
    }

    fn fields<'a>(analysis: &'a str, api: Option<&'a str>, incremental: bool, removed: &'a [String]) -> Fields<'a> {
        Fields { analysis: Some(analysis), api, api_errors: None, incremental, removed }
    }

    #[test]
    fn discovery_takes_the_top_level_public_concrete_classes_of_the_roots() {
        let a = analysis(&[
            (
                "test/p/S.scala",
                vec![
                    class("p.S", "class", (true, true, false), &["munit.FunSuite", "munit.Suite"], &["p/S.class"]),
                    class("p.Base", "class", (true, true, true), &["munit.FunSuite"], &["p/Base.class"]),
                    class("p.T", "trait", (true, true, false), &["munit.Suite"], &["p/T.class"]),
                    class("p.Hidden", "class", (true, false, false), &["munit.Suite"], &["p/Hidden.class"]),
                    class("p.S.Inner", "class", (false, true, false), &["munit.Suite"], &["p/S$Inner.class"]),
                    class("p.Z", "object", (true, true, false), &["zio.test.ZIOSpecAbstract"], &["p/Z$.class", "p/Z.class"]),
                ],
                &[],
            ),
            ("main/p/M.scala", vec![class("p.M", "class", (true, true, false), &["munit.Suite"], &["p/M.class"])], &[]),
        ]);
        let mut view = View::default();
        view.merge(&fields(&a, None, false, &[])).unwrap();
        let found: Vec<(&str, bool)> = view.candidates(&["test".to_string()]).iter().map(|c| (c.name, c.module)).collect();
        assert_eq!(found, [("p.S", false), ("p.Z", true)]);
        assert_eq!(view.candidates(&["test/".to_string()])[0].bases, ["p.S", "munit.FunSuite", "munit.Suite"]);
        assert!(view.candidates(&["tes".to_string()]).is_empty());
        assert_eq!(view.deps_missing.as_deref(), Some("the answer carried no API graph"));
    }

    #[test]
    fn the_main_classes_are_the_marked_classes_under_the_roots() {
        let a = analysis(&[
            (
                "app/src/p/Main.scala",
                vec![
                    class_with_main("p.Main", "object", (true, true, false), &["core.Entry"], &["p/Main$.class", "p/Main.class"], true),
                    class_with_main("p.run", "class", (true, true, false), &[], &["p/run.class"], true),
                    class("p.Util", "object", (true, true, false), &[], &["p/Util$.class", "p/Util.class"]),
                ],
                &[],
            ),
            ("core/src/core/Entry.scala", vec![class("core.Entry", "trait", (true, true, false), &[], &["core/Entry.class"])], &[]),
            ("other/src/q/Main.scala", vec![class_with_main("q.Main", "object", (true, true, false), &[], &["q/Main$.class", "q/Main.class"], true)], &[]),
        ]);
        let mut view = View::default();
        view.merge(&fields(&a, None, false, &[])).unwrap();
        assert_eq!(view.main_classes_under(&["app/src".to_string()]), ["p.Main", "p.run"]);
        assert_eq!(view.main_classes_under(&["app/src".to_string(), "other/src".to_string()]), ["p.Main", "p.run", "q.Main"]);
        assert!(view.main_classes_under(&["core/src".to_string()]).is_empty());
    }

    #[test]
    fn an_incremental_answer_replaces_its_files_and_drops_the_removed() {
        let full = analysis(&[
            ("t/A.scala", vec![class("p.A", "class", (true, true, false), &[], &["p/A.class"])], &[]),
            ("t/B.scala", vec![class("p.B", "class", (true, true, false), &[], &["p/B.class"])], &[]),
            ("m/C.scala", vec![class("p.C", "class", (true, true, false), &[], &["p/C.class"])], &["p/C$$anon$1.class"]),
            ("m/D.scala", vec![class("p.D", "class", (true, true, false), &[], &["p/D.class"])], &[]),
        ]);
        let graph = api(&[("t/A.scala", &[("p.A", &["p.C"])]), ("t/B.scala", &[("p.B", &["p.D"])]), ("m/C.scala", &[("p.C", &[])]), ("m/D.scala", &[("p.D", &["p.C"])])]);
        let mut view = View::default();
        view.merge(&fields(&full, Some(&graph), false, &[])).unwrap();
        assert_eq!(view.deps_missing, None);
        let set = |xs: &[&str]| xs.iter().map(|x| x.to_string()).collect::<BTreeSet<String>>();
        assert_eq!(view.affected(&set(&["p/C.class"])), set(&["p.A", "p.B", "p.C", "p.D"]));
        assert_eq!(view.affected(&set(&["p/D.class"])), set(&["p.B", "p.D"]));
        assert_eq!(view.affected(&set(&["p/C$$anon$1.class"])), set(&["p.A", "p.B", "p.C", "p.D"]));
        assert_eq!(view.affected(&set(&["p/A.class"])), set(&["p.A"]));
        // B now depends on nothing, and D is gone.
        let retyped = analysis(&[("t/B.scala", vec![class("p.B", "class", (true, true, false), &[], &["p/B.class"])], &[])]);
        let graph = api(&[("t/B.scala", &[("p.B", &[])])]);
        view.merge(&fields(&retyped, Some(&graph), true, &["m/D.scala".to_string()])).unwrap();
        assert_eq!(view.file_count(), 3);
        assert_eq!(view.affected(&set(&["p/C.class"])), set(&["p.A", "p.C"]));
        assert_eq!(view.class_count(), 3);
        // An incremental answer without its graph leaves the dependencies unknown until a full one.
        view.merge(&fields(&retyped, None, true, &[])).unwrap();
        assert!(view.deps_missing.is_some());
        view.merge(&Fields { analysis: None, api: None, api_errors: None, incremental: true, removed: &[] }).unwrap();
        assert!(view.deps_missing.is_some(), "an answer without analysis leaves the view");
        let graph = api(&[("t/A.scala", &[("p.A", &["p.C"])]), ("t/B.scala", &[]), ("m/C.scala", &[])]);
        let full = analysis(&[
            ("t/A.scala", vec![class("p.A", "class", (true, true, false), &[], &["p/A.class"])], &[]),
            ("t/B.scala", vec![class("p.B", "class", (true, true, false), &[], &["p/B.class"])], &[]),
            ("m/C.scala", vec![class("p.C", "class", (true, true, false), &[], &["p/C.class"])], &[]),
        ]);
        view.merge(&fields(&full, Some(&graph), false, &[])).unwrap();
        assert_eq!(view.deps_missing, None);
        assert!(view.heap_bytes() > 0);
    }
}
