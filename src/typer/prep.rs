//! The std's and the libraries' preparation, captured and replayed: what the type phase
//! demanded of the std and the jars, as keys that name it
//! again in another run, and a run that does all of it before the body phase, timed apart from
//! what the bodies still demand afterwards. A measurement: the replay is how long preparing the
//! std and the libraries whole before the fork would take, not a way the compiler types.
//!
//! `TEQ_PREP_CAPTURE=<file>` writes one key a line, in the order the type phase did the work:
//! `E <path>` a std file entered, `P <package>|<name>` a jar's package member entered, `C`, `V`,
//! `L` and `K` a class completed, converted from a jar, checked as a jar's, checked as the
//! std's, `S` and `B` a member's signature completed and its body typed. A class is named by its
//! package and its path through the classes, objects and givens that hold it (`prep_class_step`:
//! `scala.collection|c:List`), a member by its owner, its name and its place (`prep_member_key`),
//! the key's separators escaped in a name. What no key names is a line `? <kind> <why>: <what>`:
//! a mirror's val, which the derivation that summons it makes. The bodies of inline methods typed
//! again at each expansion are no preparation and are left out; so are local classes and their
//! members (the anonymous classes an expansion copies from a library's quoted code, typed by the
//! expanding body's worker outside the loader's lock), counted at the end, `# <count> <kind> <why>`.
//! `TEQ_PREP_REPLAY=<file>` does what the file's keys name after the signature phase, before
//! the walk or the fork, each key once, and the `--time` report says how many it found and did;
//! `TEQ_PREP_FAILED=<file>` lists the keys it found nothing for (`TEQ_PREP_WHY=1`: and where its
//! lookup stopped).

use super::Worker;
use crate::intern::Name;
use crate::symbols::*;
use crate::types::{ClassId, PkgId, SymId};
use std::sync::atomic::{AtomicBool, AtomicU8, Ordering};
use std::sync::Mutex;

static KEYS: Mutex<Vec<String>> = Mutex::new(Vec::new());
static LOCALS: Mutex<Vec<(char, &'static str, usize)>> = Mutex::new(Vec::new());
static ACTIVE: AtomicBool = AtomicBool::new(false);

/// Whether keys are captured now: `TEQ_PREP_CAPTURE` set, and the type phase under way.
#[inline]
pub fn capturing() -> bool {
    ACTIVE.load(Ordering::Relaxed)
}

/// Starts or stops the capture, where `TEQ_PREP_CAPTURE` asks for one.
pub fn capture(on: bool) {
    static STATE: AtomicU8 = AtomicU8::new(0);
    let wanted = match STATE.load(Ordering::Relaxed) {
        1 => false,
        2 => true,
        _ => {
            let w = std::env::var_os("TEQ_PREP_CAPTURE").is_some();
            STATE.store(if w { 2 } else { 1 }, Ordering::Relaxed);
            w
        }
    };
    ACTIVE.store(on && wanted, Ordering::Relaxed);
}

fn note(key: String) {
    KEYS.lock().unwrap_or_else(|e| e.into_inner()).push(key);
}

fn note_local(kind: char, why: &'static str) {
    let mut locals = LOCALS.lock().unwrap_or_else(|e| e.into_inner());
    match locals.iter_mut().find(|l| l.0 == kind && l.1 == why) {
        Some(l) => l.2 += 1,
        None => locals.push((kind, why, 1)),
    }
}

const LOCAL_CLASS: &str = "a local class";
const LOCAL_MEMBER: &str = "a local member";

/// Writes the keys captured to `TEQ_PREP_CAPTURE`'s file, and says how many of them came after
/// a replay (the residual demand) to the report.
pub fn write_capture() -> Option<usize> {
    let path = std::env::var_os("TEQ_PREP_CAPTURE")?;
    let keys = KEYS.lock().unwrap_or_else(|e| e.into_inner());
    let mut out = String::new();
    for k in keys.iter() {
        out.push_str(k);
        out.push('\n');
    }
    for (kind, why, n) in LOCALS.lock().unwrap_or_else(|e| e.into_inner()).iter() {
        out.push_str(&format!("# {} {} {}\n", n, kind, why));
    }
    let _ = std::fs::write(path, out);
    Some(keys.len())
}

/// What a replay found and did: per kind of key, how many there were and how many named
/// something of this build.
#[derive(Default, Clone)]
pub struct Replayed {
    pub kinds: Vec<(char, usize, usize)>,
}

static REPLAYED: Mutex<Option<Replayed>> = Mutex::new(None);

pub fn replayed() -> Option<Replayed> {
    REPLAYED.lock().unwrap_or_else(|e| e.into_inner()).clone()
}

/// A name as a key holds it: the key's separators escaped.
fn key_name(name: &str) -> String {
    let mut out = String::with_capacity(name.len());
    for ch in name.chars() {
        match ch {
            '%' => out.push_str("%25"),
            '#' => out.push_str("%23"),
            '/' => out.push_str("%2F"),
            '|' => out.push_str("%7C"),
            ' ' => out.push_str("%20"),
            _ => out.push(ch),
        }
    }
    out
}

fn name_of_key(text: &str) -> String {
    text.replace("%2F", "/").replace("%7C", "|").replace("%23", "#").replace("%20", " ").replace("%25", "%")
}

impl<'a> Worker<'a> {
    /// How `c` is reached from its owner: `c` a package's class or a class's nested one, `o` the
    /// class of an object a term names (an object nested in a class or trait is reached through
    /// the lazy val that holds it), `g` the class of a given defined with a body, `a` the class
    /// package `scala` makes for an arity (`arity_classes`), with the name of the step. Checked by reading: the name found
    /// leads back to `c`.
    fn prep_class_step(&self, c: ClassId) -> Result<(char, Name), &'static str> {
        let info = self.syms.class(c);
        let given_of = |s: SymId| -> bool {
            let alts = self.syms.alternatives(s).map(|a| a.to_vec()).unwrap_or_else(|| vec![s]);
            alts.iter().any(|&a| self.syms.sym(a).impl_class == Some(c))
        };
        let object_of_entry = |s: SymId| -> bool {
            let alts = self.syms.alternatives(s).map(|a| a.to_vec()).unwrap_or_else(|| vec![s]);
            alts.iter().any(|&a| self.object_of(a) == Some(c))
        };
        let held_by = info.inner_object.or(info.module_sym).map(|s| self.syms.sym(s).name);
        match info.owner {
            Owner::Package(p) => {
                let pkg = self.syms.pkg(p);
                let entry = |n: Name| pkg.entries.get(&n);
                if p == self.b.scala_pkg && self.arity_classes.get(&info.name) == Some(&c) {
                    return Ok(('a', info.name));
                }
                if entry(info.name).and_then(|e| e.class) == Some(c) {
                    return Ok(('c', info.name));
                }
                for n in held_by.into_iter().chain([info.name]) {
                    if entry(n).and_then(|e| e.term).is_some_and(|s| object_of_entry(s)) {
                        return Ok(('o', n));
                    }
                }
                if let Some(&s) = pkg.givens.iter().find(|&&s| self.syms.sym(s).impl_class == Some(c)) {
                    return Ok(('g', self.syms.sym(s).name));
                }
                Err("a package's class no entry names")
            }
            Owner::Class(o) => {
                let outer = self.syms.class(o);
                if outer.nested.get(&info.name) == Some(&c) {
                    return Ok(('c', info.name));
                }
                for n in held_by.into_iter().chain([info.name]) {
                    if outer.members.get(&n).is_some_and(|&s| object_of_entry(s)) {
                        return Ok(('o', n));
                    }
                }
                if let Some((&n, _)) = outer.members.iter().find(|(_, &s)| given_of(s)) {
                    return Ok(('g', n));
                }
                if let Some(&s) = outer.givens.iter().find(|&&s| self.syms.sym(s).impl_class == Some(c)) {
                    return Ok(('g', self.syms.sym(s).name));
                }
                // An enum's case, or an object a member of its name hides (an implicit class's
                // conversion beside an object of the name): the owner's one class of the name.
                let object = info.kind == ClassKind::Object;
                if self.prep_owned_class(o, info.name, object) == Some(c) {
                    return Ok((if object { 'm' } else { 'n' }, info.name));
                }
                Err("a nested class no member names")
            }
            Owner::Local => Err(LOCAL_CLASS),
        }
    }

    /// The one class of `name` that `o` owns, an object's or another, found by a scan of every
    /// class.
    fn prep_owned_class(&self, o: ClassId, name: Name, object: bool) -> Option<ClassId> {
        let mut found = (0..self.syms.classes.len()).map(|i| ClassId(i as u32)).filter(|&k| {
            let info = self.syms.class(k);
            info.name == name && matches!(info.owner, Owner::Class(x) if x == o) && (info.kind == ClassKind::Object) == object
        });
        let first = found.next()?;
        found.next().is_none().then_some(first)
    }

    /// The key of a class of the std or a jar, or why none names it.
    fn prep_class_key(&self, c: ClassId) -> Result<String, &'static str> {
        let (letter, name) = self.prep_class_step(c)?;
        let name = key_name(&self.name_str(name));
        match self.syms.class(c).owner {
            Owner::Package(p) => Ok(format!("{}|{}:{}", self.pkg_path(p), letter, name)),
            Owner::Class(o) => Ok(format!("{}/{}:{}", self.prep_class_key(o)?, letter, name)),
            Owner::Local => Err(LOCAL_CLASS),
        }
    }

    /// The key of a member of a class or a package of the std or a jar, or why none names it:
    /// its owner's key, its name and its place, an alternative's index among the entry's
    /// alternatives, `x<k>` the `k`-th extension method of the name, `c<k>` a constructor (the
    /// primary one `c0`, then the secondary ones).
    fn prep_member_key(&self, s: SymId) -> Result<String, &'static str> {
        let info = self.syms.sym(s);
        let place = |entry: Option<SymId>, extensions: &[SymId]| -> Option<String> {
            if let Some(e) = entry {
                if e == s {
                    return Some("0".to_string());
                }
                if let Some(k) = self.syms.alternatives(e).and_then(|alts| alts.iter().position(|&a| a == s)) {
                    return Some(k.to_string());
                }
            }
            let named: Vec<SymId> = extensions.iter().copied().filter(|&x| self.syms.sym(x).name == info.name).collect();
            named.iter().position(|&x| x == s).map(|k| format!("x{}", k))
        };
        let (owner, at) = match info.owner {
            Owner::Package(p) => {
                let entry = self.syms.pkg(p).entries.get(&info.name);
                let at = place(entry.and_then(|e| e.term), entry.map_or(&[][..], |e| &e.extensions[..]));
                (format!("{}|", self.pkg_path(p)), at)
            }
            Owner::Class(c) => {
                let class = self.syms.class(c);
                let mut at = place(class.members.get(&info.name).copied(), &class.extensions);
                if at.is_none() {
                    at = if class.primary_ctor == Some(s) {
                        Some("c0".to_string())
                    } else {
                        class.ctors.iter().position(|&k| k == s).map(|k| format!("c{}", k + 1))
                    };
                }
                (self.prep_class_key(c)?, at)
            }
            Owner::Local => return Err(LOCAL_MEMBER),
        };
        let at = at.ok_or(match info.owner {
            Owner::Package(_) => "a package member no entry names",
            _ => "a class member no entry names",
        })?;
        Ok(format!("{}#{}#{}", owner, key_name(&self.name_str(info.name)), at))
    }

    /// An owner as a reader recognises it, for a key that cannot name it.
    fn prep_described(&self, owner: Owner) -> String {
        match owner {
            Owner::Package(p) => format!("{}|", self.pkg_path(p)),
            Owner::Class(c) => {
                let info = self.syms.class(c);
                format!("{}/{:?}:{}", self.prep_described(info.owner), info.kind, self.name_str(info.name))
            }
            Owner::Local => "<local>".to_string(),
        }
    }

    fn outside_program_class(&self, c: ClassId) -> bool {
        !self.program_file(self.syms.class(c).file)
    }

    /// A class of the std or a jar was completed (`C`), converted (`V`), checked as a jar's
    /// (`L`) or as the std's (`K`).
    #[cold]
    pub(super) fn prep_note_class(&self, kind: char, c: ClassId) {
        if self.outside_program_class(c) {
            match self.prep_class_key(c) {
                Ok(k) => note(format!("{} {}", kind, k)),
                Err(why @ (LOCAL_CLASS | LOCAL_MEMBER)) => note_local(kind, why),
                Err(why) => note(format!("? {} {}: {}", kind, why, self.prep_described(Owner::Class(c)))),
            }
        }
    }

    /// A member of the std or a jar had its signature completed (`S`) or its body typed (`B`).
    #[cold]
    pub(super) fn prep_note_member(&self, kind: char, s: SymId) {
        if !self.program_file(self.syms.sym(s).file) {
            match self.prep_member_key(s) {
                Ok(k) => note(format!("{} {}", kind, k)),
                Err(why @ (LOCAL_CLASS | LOCAL_MEMBER)) => note_local(kind, why),
                Err(why) => {
                    let info = self.syms.sym(s);
                    note(format!("? {} {}: {}#{}", kind, why, self.prep_described(info.owner), self.name_str(info.name)))
                }
            }
        }
    }

    /// The std file of slot `i` was entered.
    #[cold]
    pub(super) fn prep_note_slot(&self, i: usize) {
        note(format!("E {}", self.source(self.std.slots[i].file).path));
    }

    /// The jar's member `name` of package `p` was entered.
    #[cold]
    pub(super) fn prep_note_pkg_member(&self, p: PkgId, name: Name) {
        note(format!("P {}|{}", self.pkg_path(p), key_name(&self.name_str(name))));
    }

    fn prep_pkg(&mut self, path: &str) -> Option<PkgId> {
        let mut p = ROOT_PKG;
        for seg in path.split('.').filter(|s| !s.is_empty()) {
            let name = self.interner.intern(seg);
            p = self.demand_pkg(p, name)?;
        }
        Some(p)
    }

    /// The term `name` of package `p`, entered on the way.
    fn prep_pkg_term(&mut self, p: PkgId, name: Name) -> Option<SymId> {
        if self.syms.pkg(p).entries.get(&name).and_then(|e| e.term).is_none() {
            self.load_pkg_member(p, name);
        }
        self.syms.pkg(p).entries.get(&name).and_then(|e| e.term).or_else(|| self.demand_term(p, name))
    }

    /// The entry `s` or the first of its alternatives `f` answers.
    fn prep_find<T>(&self, s: SymId, f: impl Fn(&Self, SymId) -> Option<T>) -> Option<T> {
        match self.syms.alternatives(s) {
            Some(alts) => alts.iter().find_map(|&a| f(self, a)),
            None => f(self, s),
        }
    }

    /// The class a key names in this build, entered and completed on the way.
    fn prep_class(&mut self, key: &str) -> Option<ClassId> {
        self.prep_class_why(key).ok()
    }

    /// The class a key names, or where its path found nothing.
    fn prep_class_why(&mut self, key: &str) -> Result<ClassId, String> {
        let (pkg, path) = key.split_once('|').ok_or("no package")?;
        let p = self.prep_pkg(pkg).ok_or_else(|| format!("no package {}", pkg))?;
        let mut owner: Option<ClassId> = None;
        for seg in path.split('/') {
            let (letter, text) = seg.split_once(':').ok_or("no letter")?;
            let name = self.interner.intern(&name_of_key(text));
            let impl_class = |w: &Self, s: SymId| w.syms.sym(s).impl_class;
            let object = |w: &Self, s: SymId| w.object_of(s);
            let found = match (owner, letter) {
                (None, "c") => {
                    if self.syms.pkg(p).entries.get(&name).is_none() {
                        self.load_pkg_member(p, name);
                    }
                    self.syms.pkg(p).entries.get(&name).and_then(|e| e.class).or_else(|| self.demand_class(p, name))
                }
                (None, "o") => self.prep_pkg_term(p, name).and_then(|s| self.prep_find(s, object)),
                (None, "g") => {
                    let given = self.prep_pkg_term(p, name).and_then(|s| self.prep_find(s, impl_class));
                    given.or_else(|| self.syms.pkg(p).givens.iter().find(|&&s| self.syms.sym(s).name == name).and_then(|&s| self.syms.sym(s).impl_class))
                }
                (None, "a") => self.arity_class_named(name),
                (Some(o), _) => {
                    self.complete_class(o);
                    let member = self.syms.class(o).members.get(&name).copied();
                    match letter {
                        "c" => self.syms.class(o).nested.get(&name).copied(),
                        "n" | "m" => self.prep_owned_class(o, name, letter == "m"),
                        "o" => member.and_then(|s| self.prep_find(s, object)),
                        "g" => member.and_then(|s| self.prep_find(s, impl_class)).or_else(|| {
                            self.syms.class(o).givens.iter().find(|&&s| self.syms.sym(s).name == name).and_then(|&s| self.syms.sym(s).impl_class)
                        }),
                        _ => None,
                    }
                }
                _ => None,
            };
            owner = Some(found.ok_or_else(|| format!("{}:{} absent{}", letter, text, if owner.is_some() { " in its owner" } else { "" }))?);
        }
        owner.ok_or_else(|| "empty path".to_string())
    }

    fn prep_member(&mut self, key: &str) -> Option<SymId> {
        self.prep_member_why(key).ok()
    }

    fn prep_member_why(&mut self, key: &str) -> Result<SymId, String> {
        let (rest, at) = key.rsplit_once('#').ok_or("no place")?;
        let (owner, text) = rest.rsplit_once('#').ok_or("no name")?;
        let name = self.interner.intern(&name_of_key(text));
        let (entry, extensions) = if let Some(pkg) = owner.strip_suffix('|') {
            let p = self.prep_pkg(pkg).ok_or_else(|| format!("no package {}", pkg))?;
            let entry = self.prep_pkg_term(p, name);
            (entry, self.syms.pkg(p).entries.get(&name).map(|e| e.extensions.clone()).unwrap_or_default())
        } else {
            let c = self.prep_class_why(owner).map_err(|e| format!("owner: {}", e))?;
            self.complete_class(c);
            let class = self.syms.class(c);
            if let Some(k) = at.strip_prefix('c') {
                let k: usize = k.parse().map_err(|_| "bad place")?;
                let ctor = if k == 0 { class.primary_ctor } else { class.ctors.get(k - 1).copied() };
                return ctor.ok_or_else(|| format!("constructor {} absent", k));
            }
            (class.members.get(&name).copied(), class.extensions.clone())
        };
        if let Some(k) = at.strip_prefix('x') {
            let k: usize = k.parse().map_err(|_| "bad place")?;
            return extensions.iter().copied().filter(|&x| self.syms.sym(x).name == name).nth(k).ok_or_else(|| format!("extension {} {} absent", text, k));
        }
        let k: usize = at.parse().map_err(|_| "bad place")?;
        let entry = entry.ok_or_else(|| format!("{} absent", text))?;
        match self.syms.alternatives(entry) {
            Some(alts) => alts.get(k).copied().ok_or_else(|| format!("alternative {} of {}", k, alts.len())),
            None => Ok(entry),
        }
    }

    /// A prediction of the preparation without a capture (`TEQ_PREP_PREDICT`, a diagnostic):
    /// every class of the std or a jar entered by now completed (`classes`), and the signatures
    /// of their members (`members`); what it found and did to the report, the bodies' residual
    /// demand to the capture after it.
    pub(super) fn predict_preparation(&mut self) {
        let Some(level) = std::env::var_os("TEQ_PREP_PREDICT") else { return };
        let members = level == "members";
        let n = self.syms.classes.len();
        let candidates: Vec<ClassId> = (0..n)
            .map(|i| ClassId(i as u32))
            .filter(|&c| {
                let info = self.syms.class(c);
                info.owner != Owner::Local && !self.program_file(info.file) && !matches!(info.kind, ClassKind::Builtin | ClassKind::Opaque)
            })
            .collect();
        let mut sigs = 0usize;
        for &c in &candidates {
            self.complete_class(c);
            if members {
                let info = self.syms.class(c);
                let mut syms: Vec<SymId> = Vec::new();
                for &s in info.member_order.iter().chain(info.extensions.iter()) {
                    match self.syms.alternatives(s) {
                        Some(alts) => syms.extend_from_slice(alts),
                        None => syms.push(s),
                    }
                }
                for s in syms {
                    if self.syms.sym(s).owner == Owner::Class(c) && matches!(self.syms.sym(s).kind, SymKind::Def | SymKind::Val | SymKind::Var) {
                        self.sig_of(s);
                        sigs += 1;
                    }
                }
            }
        }
        *REPLAYED.lock().unwrap_or_else(|e| e.into_inner()) = Some(Replayed { kinds: vec![('C', candidates.len(), candidates.len()), ('S', sigs, sigs), ('#', n, 0)] });
        KEYS.lock().unwrap_or_else(|e| e.into_inner()).clear();
        LOCALS.lock().unwrap_or_else(|e| e.into_inner()).clear();
    }

    /// Does what the keys of `TEQ_PREP_REPLAY`'s file name, in their order.
    pub(super) fn replay_preparation(&mut self) {
        let Some(path) = std::env::var_os("TEQ_PREP_REPLAY") else { return };
        let Ok(text) = std::fs::read_to_string(&path) else {
            eprintln!("teq: TEQ_PREP_REPLAY: cannot read {}", path.to_string_lossy());
            return;
        };
        let mut kinds: Vec<(char, usize, usize)> = Vec::new();
        let mut failed = String::new();
        let mut seen = std::collections::HashSet::new();
        for line in text.lines() {
            if line.starts_with('#') || !seen.insert(line) {
                continue;
            }
            let Some((kind, key)) = line.split_once(' ') else { continue };
            let kind = kind.chars().next().unwrap_or('?');
            let done = match kind {
                'E' => match self.std.slots.iter().position(|s| self.source(s.file).path == key) {
                    Some(i) => {
                        self.enter_std_slot(i);
                        true
                    }
                    None => false,
                },
                'P' => match key.split_once('|') {
                    Some((pkg, name)) => match self.prep_pkg(pkg) {
                        Some(p) => {
                            let name = self.interner.intern(&name_of_key(name));
                            self.load_pkg_member(p, name);
                            self.syms.pkg(p).entries.contains_key(&name)
                        }
                        None => false,
                    },
                    None => false,
                },
                'C' | 'V' | 'L' | 'K' => match self.prep_class(key) {
                    Some(c) => {
                        match kind {
                            'C' => self.complete_class(c),
                            'V' => {
                                self.convert_library_class(c);
                            }
                            'L' => self.check_library_class(c),
                            _ => self.check_class_for_run(c),
                        }
                        true
                    }
                    None => false,
                },
                'S' | 'B' => match self.prep_member(key) {
                    Some(s) => {
                        if kind == 'S' {
                            self.sig_of(s);
                        } else {
                            self.deferred_body(s);
                        }
                        true
                    }
                    None => false,
                },
                _ => false,
            };
            if !done {
                failed.push_str(line);
                if std::env::var_os("TEQ_PREP_WHY").is_some() {
                    let why = match kind {
                        'C' | 'V' | 'L' | 'K' => self.prep_class_why(key).err(),
                        'S' | 'B' => self.prep_member_why(key).err(),
                        _ => None,
                    };
                    failed.push_str(&format!(" ;; {}", why.unwrap_or_else(|| "found now".to_string())));
                }
                failed.push('\n');
            }
            match kinds.iter_mut().find(|k| k.0 == kind) {
                Some(k) => {
                    k.1 += 1;
                    k.2 += done as usize;
                }
                None => kinds.push((kind, 1, done as usize)),
            }
        }
        if let Some(out) = std::env::var_os("TEQ_PREP_FAILED") {
            let _ = std::fs::write(out, failed);
        }
        *REPLAYED.lock().unwrap_or_else(|e| e.into_inner()) = Some(Replayed { kinds });
        // What the bodies demand from here on is the residual.
        KEYS.lock().unwrap_or_else(|e| e.into_inner()).clear();
        LOCALS.lock().unwrap_or_else(|e| e.into_inner()).clear();
    }
}
