//! The record a Java source gives for one of its top-level classes: the `ClassFile` a class file would give, with
//! what the SOURCE says (the reference's `JavaParsers` read by `Namer` and `Typer`), not what javac writes: a
//! constant only where the reference reads one (`varDecl`, `tryConstant`), the parameters' names as written, no
//! `throws` and no deprecation, a field a `var` unless final. Names are resolved by Java's rules into binary names
//! (`Resolve`), descriptors are javac's erasure, `Signature` strings where a type is generic.

use super::parser::{self, Arg, Kind, Lit, Member, Mods, Param, TParam, Ty, TypeDecl, Unit};
use super::scanner::T;
use crate::classfile::{ClassFile, ConstValue, Field, Method, ACC_ABSTRACT, ACC_ANNOTATION, ACC_ENUM, ACC_FINAL, ACC_INTERFACE, ACC_PRIVATE, ACC_PROTECTED, ACC_PUBLIC, ACC_STATIC, ACC_VARARGS};

const ACC_SUPER: u16 = 0x0020;
const ACC_VOLATILE: u16 = 0x0040;
const ACC_TRANSIENT: u16 = 0x0080;
const ACC_SYNCHRONIZED: u16 = 0x0020;
const ACC_NATIVE: u16 = 0x0100;

/// A class a name resolved to.
#[derive(Clone)]
pub struct Found {
    /// Its binary name, `java/util/Map$Entry`.
    pub binary: String,
    /// Whether it is a class of the run's Scala sources, which a Java declaration may not name in this reader.
    pub run_scala: bool,
}

/// The lookups Java's name resolution needs, answered by the loader.
pub trait Resolve {
    /// The class `simple` of package `pkg` (slash-separated, `""` the unnamed package).
    fn class_in(&mut self, pkg: &str, simple: &str) -> Option<Found>;
    /// The member class `simple` of the class `outer` (a binary name).
    fn member_of(&mut self, outer: &str, simple: &str) -> Option<Found>;
    /// Whether `pkg` is a package.
    fn package(&mut self, pkg: &str) -> bool;
    /// The superclass and the interfaces of the class `binary`, as its record or class file names them; none where it
    /// is not known (or is being built: a cycle of parents stops there).
    fn parents_of(&mut self, binary: &str) -> Vec<String>;
    /// The member types the class `binary` declares, whatever their access, each by its simple and its binary name and
    /// its access flags (`ACC_PRIVATE`, `ACC_PROTECTED`, `ACC_PUBLIC`, none for a package's).
    fn member_types(&mut self, binary: &str) -> Vec<(String, String, u16)>;
    /// Whether the class `binary` is an interface (a Scala trait among them); false where it is not known.
    fn is_interface(&mut self, binary: &str) -> bool;
    /// Whether the package `pkg` (a slash path) is under one of the JDK's roots, derived from the JDK's own packages
    /// (`jdk.internal.misc`, `com.sun.nope`; not `org.nope`).
    fn jdk_rooted(&mut self, pkg: &str) -> bool;
    /// Whether the class `outer` may have a member type `simple` this reader does not see: a JDK class's (its index holds
    /// the API alone, `MethodHandleNatives.Constants` is none of it).
    fn member_unseen(&mut self, outer: &str, simple: &str) -> bool;
}

/// What the record's building found wrong, by offset in the source: a name not found, a name of the run's Scala.
pub struct Problem {
    pub offset: u32,
    pub message: String,
    /// A refusal owed only once a Scala reader asks for the class: what the reader does not know (a name it did not
    /// find, a declaration naming the run's Scala, which teq cannot type); a class no Scala source reads is javac's
    /// alone. What it found (a conflict, a parent's fault, an ambiguity of two candidates) is told at once.
    pub when_read: bool,
}

#[derive(Clone)]
enum R {
    Var(String),
    Class(String),
}

/// What a single import gives its simple name (JLS 6.4.1, 7.5.1, 7.5.3): a single-type import claims the name, found
/// or not, as a single static import does where its type holds a member type of the name or is itself not found; a
/// later scope never binds a name an import claims.
enum Claim {
    Found(Found),
    /// The import's qualifier names nothing the reader finds: told at the import, nothing more where the name is used.
    Told,
    /// The qualifier holds no type of the name: told where the name is used (`Not found: type X`), as the reference
    /// tells it (an unused one is javac's to refuse).
    Missing,
}

/// Where an on-demand import finds a simple name: among a package's classes, or a type's member types.
#[derive(Clone)]
enum Demand {
    Package(String),
    Members(String),
}

/// The unit's imports, each resolved once, with the words a message shows it by (`import java.util._`).
#[derive(Default)]
struct Imports {
    single: Vec<(String, Claim, String)>,
    demand: Vec<(Demand, String)>,
}

/// What a path of names reaches: a class, a package, or nothing at its segment `.0`, whose owner `.1` is shown as the
/// reference shows it (`<root>`, `java.util`, `object java.util.Map`), `.2` that owner's binary name where it is a class.
enum Path {
    Class(Found),
    Package(String),
    Missing(usize, String, Option<String>),
}

/// What a simple type name resolves to: a class, a name an import claims that resolves to nothing (`told` where its
/// problem is the import's), or nothing in any scope.
enum Simple {
    Found(Found),
    Claimed { told: bool },
    None,
}

/// A binary name as a message shows it: `java.util.Map.Entry`.
fn dotted(binary: &str) -> String {
    binary.replace(['/', '$'], ".")
}

struct Ctx<'u, 'r> {
    unit: &'u Unit,
    pkg: String,
    tvars: Vec<&'u TParam>,
    res: &'r mut dyn Resolve,
    problems: Vec<Problem>,
    /// The class being read and its parents, by binary name, once its header is read; whether a member's signature is
    /// being read (the class's body, where the member types its supertypes declare are in scope).
    own: String,
    own_interface: bool,
    parents: Vec<String>,
    in_body: bool,
    /// The class's own member types, by simple name and binary name; the classes it is a member of, innermost first,
    /// each by its binary name with its own member types (JLS 6.4.1: their scopes enclose the class's body).
    own_members: Vec<(String, String)>,
    outers: Vec<String>,
    /// Per class, the member types it declares or inherits (`type_members`), computed once.
    members_of: std::collections::HashMap<String, std::rc::Rc<TypeMembers>>,
    /// The classes around the one read, outermost first, and the class read: each by its binary name with its
    /// declaration, whose type parameters, member types and written supertypes `class_sig` reads (an inner member
    /// class of a generic class in scope is that class's type's member, `Outer<A>.Inner`, in a signature). Looked up
    /// from the class read outwards; no map: a nesting `d` deep would hash `d` names of up to `d` segments a class.
    scope: Vec<(String, &'u TypeDecl)>,
    /// Each written type name's class, by its place (the offset of its first name: the parser's alone, one a written
    /// type) and the part of the class it is read in (header or body): the erasure, the signature and the check of
    /// whether there is one read one name once. A reading under `quiet` (a probe whose problems are dropped) is not
    /// kept, so that the reading that holds them is a real one.
    named_memo: std::collections::HashMap<(u32, bool), R, crate::intern::FxBuild>,
    quiet: u32,
    imports: Imports,
    /// The written paths of the names that resolved to nothing, each standing in the record for the class it does not
    /// name: no check reads through one (`build`'s third result), and each has its problem held for a Scala reader.
    unresolved: Vec<String>,
}

/// The member types a class has, by simple name: its own, else those it inherits (each with its access), more than one
/// where two supertypes it inherits from declare one of the name (javac's ambiguity).
type TypeMembers = std::collections::HashMap<String, Vec<(String, u16)>>;

/// The package of a binary name (`java/util` of `java/util/Map$Entry`).
fn package_of(binary: &str) -> &str {
    binary.rsplit_once('/').map_or("", |(p, _)| p)
}

impl<'u, 'r> Ctx<'u, 'r> {
    fn is_interface(&mut self, binary: &str) -> bool {
        if binary == self.own {
            return self.own_interface;
        }
        self.res.is_interface(binary)
    }

    fn tvar(&self, name: &str) -> Option<&'u TParam> {
        self.tvars.iter().rev().find(|p| p.name == name).copied()
    }

    fn qualified(&mut self, names: &[String]) -> Option<Found> {
        match self.path(names, true) {
            Path::Class(c) => Some(c),
            _ => None,
        }
    }

    /// What the path `names` reaches by Java's reading of a qualified name (JLS 6.5.5.2): its first segments are packages
    /// up to the first that names a class there, the rest that class's member types. A path of packages alone is a
    /// package where `class` is false, else nothing at its last segment.
    fn path(&mut self, names: &[String], class: bool) -> Path {
        let mut pkg = String::new();
        for (i, n) in names.iter().enumerate() {
            if let Some(found) = self.res.class_in(&pkg, n) {
                return self.members(found, names, i + 1);
            }
            let next = if pkg.is_empty() { n.clone() } else { format!("{}/{}", pkg, n) };
            if (class && i + 1 == names.len()) || !self.res.package(&next) {
                return Path::Missing(i, if pkg.is_empty() { "<root>".to_string() } else { dotted(&pkg) }, None);
            }
            pkg = next;
        }
        Path::Package(pkg)
    }

    /// The member types of `c` that the segments of `names` from `from` on select, one in another.
    fn members(&mut self, mut c: Found, names: &[String], from: usize) -> Path {
        for (j, m) in names.iter().enumerate().skip(from) {
            match self.res.member_of(&c.binary, m) {
                Some(f) => c = f,
                None => return Path::Missing(j, format!("object {}", dotted(&c.binary)), Some(c.binary)),
            }
        }
        Path::Class(c)
    }

    /// The unit's imports, each resolved once (JLS 7.5): an import whose qualifier (the package or type it selects
    /// from) names nothing the reader finds is a problem at its last selector in the reference's words (`value nope is
    /// not a member of <root>`, E008), held for a Scala reader as every name not found is, used or not.
    fn resolve_imports(&mut self) -> Imports {
        let mut out = Imports::default();
        let unit = self.unit;
        for i in &unit.imports {
            let Some((last, q)) = i.names.split_last() else { continue };
            let shown: Vec<&str> = i.names.iter().map(|s| if s == "*" { "_" } else { s.as_str() }).collect();
            let text = format!("import {}{}", if i.is_static { "static " } else { "" }, shown.join("."));
            let at = i.at.last().copied().unwrap_or(0);
            let reached = if last != "*" && !i.is_static { self.path(&i.names, true) } else { self.path(q, false) };
            if let Path::Missing(k, owner, outer) = &reached {
                if *k < q.len() {
                    // A package under one of the JDK's roots that the JDK's index does not hold (one it holds and does
                    // not export, `jdk.internal.misc`, which the reference finds in the runtime image), or a member type
                    // this reader does not see (`member_unseen`), is this reader's gap, no error of the program: no
                    // problem at the import, its name claimed, a use not found.
                    let missing = i.names[..=*k].join("/");
                    let unseen = match outer {
                        Some(o) => self.res.member_unseen(o, &i.names[*k]),
                        None => self.res.jdk_rooted(&missing),
                    };
                    if unseen {
                        if last != "*" {
                            out.single.push((last.clone(), Claim::Missing, text));
                        }
                        continue;
                    }
                    self.held(at, format!("value {} is not a member of {}", i.names[*k], owner));
                    if last != "*" {
                        out.single.push((last.clone(), Claim::Told, text));
                    }
                    continue;
                }
            }
            match (last == "*", i.is_static, reached) {
                (true, false, Path::Package(p)) => out.demand.push((Demand::Package(p), text)),
                (true, _, Path::Class(c)) => out.demand.push((Demand::Members(c.binary), text)),
                (false, false, Path::Class(c)) => out.single.push((last.clone(), Claim::Found(c), text)),
                (false, false, _) => out.single.push((last.clone(), Claim::Missing, text)),
                (false, true, Path::Class(c)) => {
                    // a static import of members that are no type claims no type name
                    if let Some(m) = self.res.member_of(&c.binary, last) {
                        out.single.push((last.clone(), Claim::Found(m), text));
                    }
                }
                // a static import from a package (javac refuses it) gives no type name
                _ => {}
            }
        }
        out
    }

    fn held(&mut self, offset: u32, message: String) {
        self.problems.push(Problem { offset, message, when_read: true });
    }

    /// Java's resolution of a simple type name (JLS 6.4.1, 6.5.5): a type variable, a member type in scope (the class's
    /// own and inherited ones in its body, then each enclosing class's: `member_in_scope`), the unit's own types, a
    /// single-type import or a single static import, the package, an on-demand import (of a package's types or a
    /// type's static members), `java.lang`. Every candidate of one precedence is asked for: two that are distinct
    /// classes are the reference's ambiguity (E049, `Typer.findRef`), reported at the name (`at`; an annotation's,
    /// which has no place here, is not), the first taken. A name a single import claims goes no further, found or not.
    fn simple(&mut self, n: &str, at: Option<u32>) -> Simple {
        if let Some(f) = self.member_in_scope(n, at) {
            return Simple::Found(f);
        }
        if self.unit.types.iter().any(|t| t.name == n) {
            return Simple::Found(Found { binary: if self.pkg.is_empty() { n.to_string() } else { format!("{}/{}", self.pkg, n) }, run_scala: false });
        }
        let mut single = Vec::new();
        let mut claimed = None;
        for (name, claim, text) in &self.imports.single {
            if name != n {
                continue;
            }
            match claim {
                Claim::Found(f) => single.push((f.clone(), text.clone())),
                Claim::Told => claimed = Some(true),
                Claim::Missing => claimed = Some(claimed.unwrap_or(false)),
            }
        }
        if let Some(f) = self.one_of(n, at, "imported by name", single) {
            return Simple::Found(f);
        }
        if let Some(told) = claimed {
            return Simple::Claimed { told };
        }
        let pkg = self.pkg.clone();
        if let Some(c) = self.res.class_in(&pkg, n) {
            return Simple::Found(c);
        }
        let mut demand = Vec::new();
        for (d, text) in self.imports.demand.clone() {
            let found = match &d {
                Demand::Package(p) => self.res.class_in(p, n),
                Demand::Members(b) => self.res.member_of(b, n),
            };
            if let Some(f) = found {
                demand.push((f, text));
            }
        }
        if let Some(f) = self.one_of(n, at, "imported", demand) {
            return Simple::Found(f);
        }
        match self.res.class_in("java/lang", n) {
            Some(f) => Simple::Found(f),
            None => Simple::None,
        }
    }

    /// The member types the class `binary` declares, whatever their access, and for each name it does not declare,
    /// those it inherits from its direct supertypes (JLS 8.5): a supertype's member type is inherited unless it is
    /// private, or of package access in another package; one a nearer supertype declares hides those of its own
    /// supertypes. A cycle of parents stops where it comes back.
    fn type_members(&mut self, binary: &str, walk: &mut Vec<String>) -> std::rc::Rc<TypeMembers> {
        if let Some(m) = self.members_of.get(binary) {
            return m.clone();
        }
        if walk.iter().any(|w| w == binary) {
            return std::rc::Rc::new(TypeMembers::new());
        }
        walk.push(binary.to_string());
        let own: Vec<(String, String, u16)> = self.res.member_types(binary);
        let parents = self.res.parents_of(binary);
        let out = self.with_inherited(binary, own, &parents, walk);
        walk.pop();
        let out = std::rc::Rc::new(out);
        self.members_of.insert(binary.to_string(), out.clone());
        out
    }

    /// `own`, the member types `binary` declares, with those it inherits from `parents` for every other name.
    fn with_inherited(&mut self, binary: &str, own: Vec<(String, String, u16)>, parents: &[String], walk: &mut Vec<String>) -> TypeMembers {
        let mut out = TypeMembers::new();
        for (simple, b, acc) in own {
            out.entry(simple).or_default().push((b, acc));
        }
        let declared: Vec<String> = out.keys().cloned().collect();
        let pkg = package_of(binary).to_string();
        for p in parents {
            let theirs = self.type_members(p, walk);
            for (simple, list) in theirs.iter() {
                if declared.contains(simple) {
                    continue;
                }
                for (b, acc) in list {
                    let inherited = acc & ACC_PRIVATE == 0 && (acc & (ACC_PUBLIC | ACC_PROTECTED) != 0 || package_of(b.split('$').next().unwrap_or(b)) == pkg);
                    let entry = out.entry(simple.clone()).or_default();
                    if inherited && !entry.iter().any(|(x, _)| x == b) {
                        entry.push((b.clone(), *acc));
                    }
                }
            }
        }
        out.retain(|_, v| !v.is_empty());
        out
    }

    /// The member type `n` in scope at a name of the class's body or of a class it is a member of (JLS 6.4.1): the
    /// class's own or inherited ones, then each enclosing class's, innermost first; two that the same class inherits
    /// from two supertypes are javac's ambiguity, told at the name (`at`) in the reference's words (E049), the first
    /// taken. `None` where no class in scope has one.
    fn member_in_scope(&mut self, n: &str, at: Option<u32>) -> Option<Found> {
        self.member_in_scope_of(n, at).map(|(f, _)| f)
    }

    /// The member type a simple name names in scope, with the class around it in whose scope it is found (its own or
    /// an inherited member of that class).
    fn member_in_scope_of(&mut self, n: &str, at: Option<u32>) -> Option<(Found, String)> {
        let mut scopes: Vec<(String, std::rc::Rc<TypeMembers>)> = Vec::new();
        if self.in_body {
            let own = self.own.clone();
            let members = match self.members_of.get(&own) {
                Some(m) => m.clone(),
                None => {
                    let declared: Vec<(String, String, u16)> = self.own_members.iter().map(|(s, b)| (s.clone(), b.clone(), ACC_PUBLIC)).collect();
                    let parents = self.parents.clone();
                    let m = std::rc::Rc::new(self.with_inherited(&own, declared, &parents, &mut vec![own.clone()]));
                    self.members_of.insert(own.clone(), m.clone());
                    m
                }
            };
            scopes.push((own, members));
        }
        for o in self.outers.clone() {
            let m = self.type_members(&o, &mut Vec::new());
            scopes.push((o, m));
        }
        for (class, members) in scopes {
            let Some(list) = members.get(n) else { continue };
            let (first, _) = list[0].clone();
            if let (Some(at), Some((second, _))) = (at, list.iter().find(|(b, _)| *b != first)) {
                let mut owner = |b: &str| {
                    let outer = b.rsplit_once('$').map_or(b, |(o, _)| o);
                    format!("{} {}", if self.res.is_interface(outer) { "trait" } else { "class" }, dotted(outer))
                };
                let (o1, o2) = (owner(&first), owner(second));
                self.problems.push(Problem {
                    offset: at,
                    message: format!("Reference to {} is ambiguous. It is both inherited in {} and inherited subsequently in {}", n, o1, o2),
                    when_read: false,
                });
            }
            return Some((Found { binary: first, run_scala: false }, class));
        }
        None
    }

    /// The first of the candidates of one precedence, the import that brought each; a second that is another class
    /// is the ambiguity, reported at `at` in the words teq gives a Scala import's (`kind`: imported by name, or
    /// imported); the same class twice is none.
    fn one_of(&mut self, n: &str, at: Option<u32>, kind: &str, found: Vec<(Found, String)>) -> Option<Found> {
        let mut found = found.into_iter();
        let (first, first_text) = found.next()?;
        if let (Some(at), Some((_, second_text))) = (at, found.find(|(f, _)| f.binary != first.binary)) {
            self.problems.push(Problem {
                offset: at,
                message: format!("reference to {} is ambiguous: it is both {} by {} and {} subsequently by {}", n, kind, first_text, kind, second_text),
                when_read: false,
            });
        }
        Some(first)
    }

    /// The class a written type name names, or its type variable. A name that names nothing binds nothing: its
    /// problem is held at the segment that names nothing, in the reference's words (`Not found: type X`, `type Missing
    /// is not a member of object p.Outer`, `value nope is not a member of java`), and the record holds its written
    /// path (`unresolved`), which no check reads through.
    fn named(&mut self, segs: &[(String, Vec<Arg>, u32)], at: u32) -> R {
        if segs.len() == 1 && self.tvar(&segs[0].0).is_some() {
            return R::Var(segs[0].0.clone());
        }
        let key = (at, self.in_body);
        if let Some(r) = self.named_memo.get(&key) {
            return r.clone();
        }
        let r = self.named_class(segs, at);
        if self.quiet == 0 {
            self.named_memo.insert(key, r.clone());
        }
        r
    }

    fn named_class(&mut self, segs: &[(String, Vec<Arg>, u32)], at: u32) -> R {
        let names: Vec<String> = segs.iter().map(|s| s.0.clone()).collect();
        let reached = match self.simple(&segs[0].0, Some(at)) {
            Simple::Found(c) => self.members(c, &names, 1),
            Simple::Claimed { told } => {
                if !told {
                    self.held(at, format!("Not found: type {}", names[0]));
                }
                return self.unresolved(&names);
            }
            Simple::None if names.len() == 1 => {
                self.held(at, format!("Not found: type {}", names[0]));
                return self.unresolved(&names);
            }
            Simple::None => self.path(&names, true),
        };
        let c = match reached {
            Path::Class(c) => c,
            Path::Missing(0, ..) | Path::Package(_) => {
                self.held(at, format!("Not found: {}", names[0]));
                return self.unresolved(&names);
            }
            Path::Missing(k, owner, _) => {
                let kind = if k + 1 == names.len() { "type" } else { "value" };
                self.held(segs[k].2, format!("{} {} is not a member of {}", kind, names[k], owner));
                return self.unresolved(&names);
            }
        };
        if c.run_scala {
            self.held(at, format!("not supported yet: a Java declaration naming {}, a class of this program's Scala sources", names.join(".")));
        }
        R::Class(c.binary)
    }

    fn unresolved(&mut self, names: &[String]) -> R {
        let path = names.join("/");
        if !self.unresolved.contains(&path) {
            self.unresolved.push(path.clone());
        }
        R::Class(path)
    }

    fn erase(&mut self, t: &Ty) -> String {
        match t {
            Ty::Prim(p) => prim(*p).to_string(),
            Ty::Void => "V".to_string(),
            Ty::Array(e) | Ty::Repeated(e) => format!("[{}", self.erase(e)),
            Ty::Error => "Ljava/lang/Object;".to_string(),
            Ty::Named(segs, at) => match self.named(segs, *at) {
                R::Var(v) => {
                    let bound = self.tvar(&v).and_then(|p| p.bounds.first().cloned());
                    match bound {
                        // the variable's first bound, erased: what javac writes (JLS 4.6)
                        Some(b) => {
                            let saved = std::mem::take(&mut self.tvars);
                            // a bound naming its own variable (`T extends Comparable<T>`) erases to the class
                            self.tvars = saved.iter().copied().filter(|p| p.name != v).collect();
                            let out = self.erase(&b);
                            self.tvars = saved;
                            out
                        }
                        None => "Ljava/lang/Object;".to_string(),
                    }
                }
                R::Class(c) => format!("L{};", c),
            },
        }
    }

    fn sig(&mut self, t: &Ty) -> String {
        match t {
            Ty::Prim(p) => prim(*p).to_string(),
            Ty::Void => "V".to_string(),
            Ty::Array(e) | Ty::Repeated(e) => format!("[{}", self.sig(e)),
            Ty::Error => "Ljava/lang/Object;".to_string(),
            Ty::Named(segs, at) => match self.named(segs, *at) {
                R::Var(v) => format!("T{};", v),
                R::Class(c) => self.class_sig(&c, segs),
            },
        }
    }

    /// The signature of the class `c` a written path `segs` names, javac's form: `Lp/C<args>;`, or, where a class
    /// around `c` is parameterized (written, `J.A<String>.B<String>`, or its type parameters in scope, `Inner` in the
    /// body of `Outer<A>`), `Lp/J$A<..>.B<..>;` from that class on.
    fn class_sig(&mut self, c: &str, segs: &[(String, Vec<Arg>, u32)]) -> String {
        // the class and those around it, outermost first, each with its written arguments (the segments align from
        // the end) or, written by a simple name in scope, its own type parameters where `c` is an inner member of it
        let mut chain: Vec<String> = Vec::new();
        let mut at = c.to_string();
        loop {
            chain.push(at.clone());
            match at.rsplit_once('$') {
                Some((outer, _)) if !outer.ends_with('/') => at = outer.to_string(),
                _ => break,
            }
        }
        chain.reverse();
        let n = chain.len();
        let mut args: Vec<Option<String>> = vec![None; n];
        for (k, seg) in segs.iter().rev().enumerate() {
            if k >= n {
                break;
            }
            if !seg.1.is_empty() {
                let mut s = String::from("<");
                for a in &seg.1 {
                    s.push_str(&self.arg_sig(a));
                }
                s.push('>');
                args[n - 1 - k] = Some(s);
            }
        }
        // The first segment a simple name of a member type in scope, inner up to the last (`Inner`, `Outer<U>.Nested<V>`
        // in the body of `Gen<T>`): javac's outer type of it, the classes around it with their own type variables where
        // it is an inner member of one in scope (`Gen<T>.Outer<U>.Nested<V>`), or, where it is inherited, the
        // supertype of the class in whose scope it is found that declares it, as written there (`asOuterSuper`:
        // `Outer<String>.Inner` in the body of `class Child extends Outer<String>`).
        let mut prefix: Option<(usize, String)> = None;
        if !segs.is_empty() && segs.len() <= n && n >= 2 {
            let j0 = n - segs.len();
            // a later class an inner member of the one before it: javac takes a written argument to say so (a static
            // member is not selected from a parameterized type)
            let inner_after = (j0 + 1..n).all(|k| args[k - 1].is_some() || self.inner_member(&chain[k - 1], &chain[k]) == Some(true));
            let mut j = j0;
            while inner_after && j > 0 {
                let (outer, simple) = (chain[j - 1].clone(), chain[j].rsplit('$').next().unwrap_or("").to_string());
                let Some(od) = self.scope_class(&outer) else {
                    if j == j0 && self.inner_member(&outer, &chain[j]) == Some(true) {
                        prefix = self.inherited_outer(&segs[0].0, &outer).map(|p| (j0, p));
                    }
                    break;
                };
                let inner = od.members.iter().any(|m| matches!(m, Member::Type(t) if t.name == simple && !is_static_member(t, od)));
                if !inner {
                    break;
                }
                if !od.tparams.is_empty() && args[j - 1].is_none() {
                    args[j - 1] = Some(format!("<{}>", od.tparams.iter().map(|t| format!("T{};", t.name)).collect::<String>()));
                }
                j -= 1;
            }
        }
        let first = (0..n - 1).find(|&i| args[i].is_some());
        let mut s = String::from("L");
        match (prefix, first) {
            (Some((j0, p)), _) => {
                s.push_str(&p);
                for j in j0..n {
                    s.push('.');
                    s.push_str(chain[j].rsplit('$').next().unwrap_or(""));
                    s.push_str(args[j].as_deref().unwrap_or(""));
                }
            }
            (None, None) => {
                s.push_str(c);
                s.push_str(args[n - 1].as_deref().unwrap_or(""));
            }
            (None, Some(i)) => {
                s.push_str(&chain[i]);
                s.push_str(args[i].as_deref().unwrap_or(""));
                for j in i + 1..n {
                    s.push('.');
                    s.push_str(chain[j].rsplit('$').next().unwrap_or(""));
                    s.push_str(args[j].as_deref().unwrap_or(""));
                }
            }
        }
        s.push(';');
        s
    }

    /// The declaration of the class `binary` where it is the class read or one around it.
    fn scope_class(&self, binary: &str) -> Option<&'u TypeDecl> {
        self.scope.iter().rev().find(|(b, _)| b == binary).map(|&(_, d)| d)
    }

    /// Whether the class `inner` is an inner member class (not static) of `outer`; `None` where its member types are
    /// not known (the JDK index holds its API alone).
    fn inner_member(&mut self, outer: &str, inner: &str) -> Option<bool> {
        let simple = inner.rsplit('$').next().unwrap_or("");
        if let Some(od) = self.scope_class(outer) {
            return od.members.iter().find_map(|m| match m {
                Member::Type(t) if t.name == simple => Some(!is_static_member(t, od)),
                _ => None,
            });
        }
        self.res.member_types(outer).iter().find(|(m, b, _)| m == simple && b == inner).map(|(_, _, a)| a & ACC_STATIC == 0)
    }

    /// The outer type, javac's signature without its `L` and `;`, of an inner member class of `owner` the simple name `n`
    /// names where a class around the one read inherits it: that class's supertype of the class `owner`, as it is
    /// written there and only where it is parameterized; none where `owner` is not one of its written supertypes (a
    /// class between them), or where reading it here holds a problem (it is read where it is written, its own record).
    fn inherited_outer(&mut self, n: &str, owner: &str) -> Option<String> {
        let (_, class) = self.member_in_scope_of(n, None)?;
        let d = self.scope_class(&class)?;
        let parents: Vec<&'u Ty> = d.superclass.iter().chain(d.interfaces.iter()).collect();
        let (problems, unresolved) = (self.problems.len(), self.unresolved.len());
        let mut out = None;
        self.quiet += 1;
        for t in parents {
            let Ty::Named(psegs, at) = t else { continue };
            if matches!(self.named(psegs, *at), R::Class(b) if b == owner) {
                let sig = self.class_sig(owner, psegs);
                out = sig.contains('<').then(|| sig.trim_start_matches('L').trim_end_matches(';').to_string());
                break;
            }
        }
        self.quiet -= 1;
        let quiet = self.problems.len() == problems && self.unresolved.len() == unresolved;
        self.problems.truncate(problems);
        self.unresolved.truncate(unresolved);
        out.filter(|_| quiet)
    }

    fn arg_sig(&mut self, a: &Arg) -> String {
        match a {
            Arg::Ty(t) => self.sig(t),
            Arg::Wild(Some(hi), _) => format!("+{}", self.sig(hi)),
            Arg::Wild(None, Some(lo)) => format!("-{}", self.sig(lo)),
            Arg::Wild(None, None) => "*".to_string(),
        }
    }

    /// Whether a written type has a signature of its own, javac's rule: its signature is not its erasure (a type
    /// variable, a type argument, an inner member class of a parameterized type).
    fn generic(&mut self, t: &Ty) -> bool {
        // the type's problems are held where it is read for the record (its erasure, its signature): not again here
        let problems = self.problems.len();
        self.quiet += 1;
        let sig = self.sig(t);
        let differs = sig != self.erase(t);
        self.quiet -= 1;
        self.problems.truncate(problems);
        differs
    }

    fn tparams_sig(&mut self, ps: &[TParam]) -> String {
        if ps.is_empty() {
            return String::new();
        }
        let mut s = "<".to_string();
        for p in ps {
            s.push_str(&p.name);
            if p.bounds.is_empty() {
                s.push_str(":Ljava/lang/Object;");
            }
            for (i, b) in p.bounds.iter().enumerate() {
                let sig = self.sig(b);
                // a class bound, then the interface bounds (JVMS 4.7.9.1): a first bound that is an interface leaves the
                // class bound empty, `T::Ljava/lang/Comparable<TT;>;`, as javac writes it
                let class = sig.strip_prefix('L').map(|r| r.split(['<', ';']).next().unwrap_or(r).to_string());
                s.push(':');
                if i == 0 && class.is_some_and(|b| self.is_interface(&b)) {
                    s.push(':');
                }
                s.push_str(&sig);
            }
        }
        s.push('>');
        s
    }
}

fn prim(t: T) -> &'static str {
    match t {
        T::Boolean => "Z",
        T::Byte => "B",
        T::Short => "S",
        T::Char => "C",
        T::Int => "I",
        T::Long => "J",
        T::Float => "F",
        T::Double => "D",
        _ => "V",
    }
}

/// A written Java type in Scala's spelling, as a message shows a member's type: `Int`, `Array[Int]`,
/// `java.util.List[String]`, `?`, `? <: T`, `? >: T`; a repeated parameter's `T*`.
fn scala_text(t: &Ty) -> String {
    match t {
        Ty::Prim(p) => match p {
            T::Boolean => "Boolean",
            T::Byte => "Byte",
            T::Short => "Short",
            T::Char => "Char",
            T::Int => "Int",
            T::Long => "Long",
            T::Float => "Float",
            _ => "Double",
        }
        .to_string(),
        Ty::Void => "Unit".to_string(),
        Ty::Array(e) => format!("Array[{}]", scala_text(e)),
        Ty::Repeated(e) => format!("{}*", scala_text(e)),
        Ty::Error => "<error>".to_string(),
        Ty::Named(segs, _) => segs
            .iter()
            .map(|(n, args, _)| {
                if args.is_empty() {
                    return n.clone();
                }
                let args: Vec<String> = args
                    .iter()
                    .map(|a| match a {
                        Arg::Ty(t) => scala_text(t),
                        Arg::Wild(Some(hi), _) => format!("? <: {}", scala_text(hi)),
                        Arg::Wild(None, Some(lo)) => format!("? >: {}", scala_text(lo)),
                        Arg::Wild(None, None) => "?".to_string(),
                    })
                    .collect();
                format!("{}[{}]", n, args.join(", "))
            })
            .collect::<Vec<_>>()
            .join("."),
    }
}

fn access(m: &Mods) -> u16 {
    let f = m.flags;
    let mut a = 0;
    if f & parser::PUBLIC != 0 {
        a |= ACC_PUBLIC;
    }
    if f & parser::PROTECTED != 0 {
        a |= ACC_PROTECTED;
    }
    if f & parser::PRIVATE != 0 {
        a |= ACC_PRIVATE;
    }
    if f & parser::STATIC != 0 {
        a |= ACC_STATIC;
    }
    if f & parser::FINAL != 0 {
        a |= ACC_FINAL;
    }
    if f & parser::ABSTRACT != 0 {
        a |= ACC_ABSTRACT;
    }
    a
}

/// A constant converted to the type written (`forConst`'s `const.convertTo`): `None` where the conversion fails
/// (`byte b = 128`), a String only where the type is written `String`.
fn constant(l: &Lit, ty: &Ty) -> Option<ConstValue> {
    if let Lit::Str(s) = l {
        return matches!(ty, Ty::Named(segs, _) if segs.len() == 1 && segs[0].0 == "String" && segs[0].1.is_empty()).then(|| ConstValue::Str(s.clone()));
    }
    let Ty::Prim(p) = ty else { return None };
    // the literal as a number (a char's code; a boolean only to boolean)
    let (int, long, float, double, is_bool, b) = match l {
        Lit::Bool(b) => (None, None, None, None, true, *b),
        Lit::Char(c) => {
            // a lone surrogate's stand-in (`crate::text`) is the unit it stands for
            let unit = crate::text::surrogate_of(*c).map_or(*c as u32, u32::from);
            (Some(unit as i64), Some(unit as i64), Some(unit as f64), Some(unit as f64), false, false)
        }
        Lit::Int(v) => (Some(*v as i64), Some(*v as i64), Some(*v as f64), Some(*v as f64), false, false),
        Lit::Long(v) => (None, Some(*v), Some(*v as f64), Some(*v as f64), false, false),
        Lit::Float(v) => (None, None, Some(*v as f64), Some(*v as f64), false, false),
        Lit::Double(v) => (None, None, None, Some(*v), false, false),
        Lit::Str(_) => unreachable!(),
    };
    match p {
        T::Boolean => is_bool.then(|| ConstValue::Int(b as i32)),
        _ if is_bool => None,
        T::Byte => int.filter(|v| i8::try_from(*v).is_ok()).map(|v| ConstValue::Int(v as i32)),
        T::Short => int.filter(|v| i16::try_from(*v).is_ok()).map(|v| ConstValue::Int(v as i32)),
        T::Char => int.filter(|v| u16::try_from(*v).is_ok()).map(|v| ConstValue::Int(v as i32)),
        T::Int => int.filter(|v| i32::try_from(*v).is_ok()).map(|v| ConstValue::Int(v as i32)),
        T::Long => long.map(ConstValue::Long),
        T::Float => float.map(|v| ConstValue::Float(v as f32)),
        T::Double => double.map(ConstValue::Double),
        _ => None,
    }
}

/// The record of `decl`, a top-level class or interface of `unit` (the forms past this reader are refused where the
/// source is entered), its problems, and the written paths of the names it did not resolve, which a check of the
/// record's parents or members skips (each with its problem held for a Scala reader).
/// Whether the member type `decl` of `outer` is static (JLS 8.1.3, 8.5.1, 9.5): written so, an enum, a record, an
/// interface or an annotation declaration, or a member of an interface.
pub fn is_static_member(decl: &TypeDecl, outer: &TypeDecl) -> bool {
    decl.mods.flags & parser::STATIC != 0
        || matches!(decl.kind, Kind::Enum | Kind::Record | Kind::Interface | Kind::Annotation)
        || matches!(outer.kind, Kind::Interface | Kind::Annotation)
}

/// The access flags of the member type `decl` of `outer` in its `InnerClasses` entry: as written, public in an
/// interface, static where it is (`is_static_member`).
pub fn member_access(decl: &TypeDecl, outer: &TypeDecl) -> u16 {
    let mut a = access(&decl.mods) & (ACC_PUBLIC | ACC_PROTECTED | ACC_PRIVATE | ACC_FINAL | ACC_ABSTRACT);
    if matches!(outer.kind, Kind::Interface | Kind::Annotation) {
        a |= ACC_PUBLIC;
    }
    if is_static_member(decl, outer) {
        a |= ACC_STATIC;
    }
    if matches!(decl.kind, Kind::Interface | Kind::Annotation) {
        a |= ACC_INTERFACE | ACC_ABSTRACT;
    }
    a
}

/// The record of `decl`, a top-level declaration of `unit` (`outers` empty) or the member type of the last of
/// `outers` (the declarations it is nested in, the top-level one first): its binary name `p/O$M$N`.
pub fn build(unit: &Unit, outers: &[&TypeDecl], decl: &TypeDecl, text: &str, res: &mut dyn Resolve) -> (ClassFile, Vec<Problem>, Vec<String>) {
    let pkg = unit.package.join("/");
    let mut binaries: Vec<String> = Vec::new();
    for o in outers {
        let b = match binaries.last() {
            Some(prev) => format!("{}${}", prev, o.name),
            None if pkg.is_empty() => o.name.clone(),
            None => format!("{}/{}", pkg, o.name),
        };
        binaries.push(b);
    }
    let own = match binaries.last() {
        Some(prev) => format!("{}${}", prev, decl.name),
        None if pkg.is_empty() => decl.name.clone(),
        None => format!("{}/{}", pkg, decl.name),
    };
    // An inner class (a member class that is not static) takes its outer instance first in its constructors'
    // descriptors, as javac writes them (the loader drops it, `sig::member_sig`).
    // The class's access as its implicit constructors take it: a member's as a member (public in an interface).
    let class_access = match outers.last() {
        Some(od) => member_access(decl, od) & (ACC_PUBLIC | ACC_PROTECTED | ACC_PRIVATE),
        None => access(&decl.mods) & (ACC_PUBLIC | ACC_PROTECTED | ACC_PRIVATE),
    };
    let inner_outer = match (binaries.last(), outers.last()) {
        (Some(o), Some(od)) if decl.kind == Kind::Class && !is_static_member(decl, od) => Some(o.clone()),
        _ => None,
    };
    // An inner class (a member that is not static) sees the type variables of the classes around it, up to the first
    // static one.
    let mut tvars: Vec<&TParam> = Vec::new();
    let mut member = decl;
    for o in outers.iter().rev() {
        if is_static_member(member, o) {
            break;
        }
        tvars.extend(o.tparams.iter());
        member = o;
    }
    tvars.extend(decl.tparams.iter());
    let own_members: Vec<(String, String)> = decl
        .members
        .iter()
        .filter_map(|m| match m {
            Member::Type(t) => Some((t.name.clone(), format!("{}${}", own, t.name))),
            _ => None,
        })
        .collect();
    let mut c = Ctx {
        unit,
        pkg: pkg.clone(),
        tvars,
        res,
        problems: Vec::new(),
        own: own.clone(),
        own_interface: matches!(decl.kind, Kind::Interface | Kind::Annotation),
        parents: Vec::new(),
        in_body: false,
        own_members,
        outers: binaries.iter().rev().cloned().collect(),
        members_of: std::collections::HashMap::new(),
        scope: binaries.iter().cloned().zip(outers.iter().copied()).chain(std::iter::once((own.clone(), decl))).collect(),
        named_memo: std::collections::HashMap::default(),
        quiet: 0,
        imports: Imports::default(),
        unresolved: Vec::new(),
    };
    c.imports = c.resolve_imports();
    // an annotation declaration is an interface (javac's ACC_INTERFACE): implemented, never extended
    let interface = matches!(decl.kind, Kind::Interface | Kind::Annotation);
    let mut cf = ClassFile { name: own.clone(), ..Default::default() };
    cf.access = (access(&decl.mods) & (ACC_PUBLIC | ACC_FINAL | ACC_ABSTRACT)) | if interface { ACC_INTERFACE | ACC_ABSTRACT } else { ACC_SUPER };
    // A member type's `InnerClasses` entries, javac's: its own (with its outer, its simple name and its access as a
    // member: static where it is), and one for each member type it declares.
    if let (Some(outer), Some(outer_decl)) = (binaries.last(), outers.last()) {
        let a = member_access(decl, outer_decl);
        if a & (ACC_PUBLIC | ACC_PROTECTED) != 0 {
            cf.access |= ACC_PUBLIC;
        }
        cf.inner_classes.push(crate::classfile::InnerClass { inner: own.clone(), outer: Some(outer.clone()), name: Some(decl.name.clone()), access: a });
    }
    for m in &decl.members {
        if let Member::Type(t) = m {
            cf.inner_classes.push(crate::classfile::InnerClass { inner: format!("{}${}", own, t.name), outer: Some(own.clone()), name: Some(t.name.clone()), access: member_access(t, decl) });
        }
    }
    cf.source_file = None;
    // A record is final and extends java.lang.Record (`recordDecl`: `javaLangRecord()`, `Flags.Final`).
    let record = decl.kind == Kind::Record;
    // An enum extends java.lang.Enum<E> (`enumDecl`'s `superclazz`), marked ACC_ENUM as javac marks it; it is not
    // final, as the reference reads it (`mods | Flags.JavaEnum`: a constant's body is skipped).
    let java_enum = decl.kind == Kind::Enum;
    let root = if record {
        "java/lang/Record"
    } else if java_enum {
        "java/lang/Enum"
    } else {
        "java/lang/Object"
    };
    if record {
        cf.access |= ACC_FINAL;
    }
    if java_enum {
        cf.access |= ACC_ENUM;
    }
    // `sealed` is read, its `permits` is not (`modifiers`; `permittedSubclasses` is never used): the children are the
    // run's sources that extend it (`build_java_source_records`); `non-sealed` is read and dropped.
    cf.sealed = decl.mods.flags & parser::SEALED != 0;
    cf.superclass = Some(match &decl.superclass {
        Some(t) if !interface => match c.erase(t).strip_prefix('L').and_then(|s| s.strip_suffix(';')) {
            Some(b) => b.to_string(),
            None => root.to_string(),
        },
        _ => root.to_string(),
    });
    let ifaces: Vec<Ty> = decl.interfaces.clone();
    cf.interfaces = ifaces.iter().filter_map(|t| c.erase(t).strip_prefix('L').and_then(|s| s.strip_suffix(';')).map(str::to_string)).collect();
    // An annotation declaration is javac's ACC_ANNOTATION interface extending java.lang.annotation.Annotation
    // (`annotationDecl`'s parents): the loader marks it and its defaulted elements, and the typer applies the
    // constructor it makes of them (`type_java_annotation`) as for one read from a class file.
    if decl.kind == Kind::Annotation {
        cf.access |= ACC_ANNOTATION;
        cf.interfaces.push("java/lang/annotation/Annotation".to_string());
    }
    let class_generic = java_enum || !decl.tparams.is_empty() || decl.superclass.iter().chain(&ifaces).any(|t| c.generic(t));
    if class_generic {
        let mut s = c.tparams_sig(&decl.tparams.clone());
        match &decl.superclass {
            Some(t) if !interface => s.push_str(&c.sig(t)),
            _ if java_enum => s.push_str(&format!("Ljava/lang/Enum<L{};>;", cf.name)),
            _ => s.push_str(&format!("L{};", root)),
        }
        for t in &ifaces {
            s.push_str(&c.sig(t));
        }
        cf.signature = Some(s);
    }
    cf.annotations = annotations(&mut c, &decl.mods);
    // the header read (its parents, as scalac and javac bind them, outside the class's scope), the members' signatures
    // are the body's
    c.parents = cf.superclass.iter().chain(&cf.interfaces).cloned().collect();
    c.in_body = true;
    // An enum's constants, its static final fields of its own type (`enumConst`: a `JavaEnumValue`, its arguments and
    // its body skipped), and its `values()` and `valueOf(String)` (`enumDecl`'s `predefs`).
    if java_enum {
        let own = format!("L{};", cf.name);
        for (name, _) in &decl.constants {
            cf.fields.push(Field { access: ACC_PUBLIC | ACC_STATIC | ACC_FINAL | ACC_ENUM, name: name.clone(), descriptor: own.clone(), signature: None, constant: None, deprecated: false, annotations: Vec::new() });
        }
        cf.methods.push(Method { access: ACC_PUBLIC | ACC_STATIC, name: "values".into(), descriptor: format!("()[{}", own), param_names: Some(Vec::new()), ..Default::default() });
        cf.methods.push(Method { access: ACC_PUBLIC | ACC_STATIC, name: "valueOf".into(), descriptor: format!("(Ljava/lang/String;){}", own), param_names: Some(vec![Some("x".to_string())]), ..Default::default() });
    }
    // A record's components are its private final fields (javac's; the reference's header is its canonical
    // constructor's parameters and its accessors' results).
    if record {
        for p in &decl.header {
            let ty = component_type(&p.ty);
            let descriptor = c.erase(&ty);
            let signature = c.generic(&ty).then(|| c.sig(&ty));
            cf.fields.push(Field { access: ACC_PRIVATE | ACC_FINAL, name: p.name.clone(), descriptor, signature, constant: None, deprecated: false, annotations: Vec::new() });
        }
    }
    let mut ctor = false;
    for m in &decl.members {
        match m {
            Member::Field { mods, ty, name, constant: lit, .. } => {
                let mut a = access(mods);
                if interface {
                    a |= ACC_PUBLIC | ACC_STATIC | ACC_FINAL;
                }
                if mods.flags & parser::VOLATILE != 0 {
                    a |= ACC_VOLATILE;
                }
                if mods.flags & parser::TRANSIENT != 0 {
                    a |= ACC_TRANSIENT;
                }
                let descriptor = c.erase(ty);
                let signature = c.generic(ty).then(|| c.sig(ty));
                // a constant only of a static final field (`varDecl` 763), as the literal converts to its type
                let constant = lit.as_ref().filter(|_| a & ACC_STATIC != 0 && a & ACC_FINAL != 0).and_then(|l| constant(l, ty));
                let annotations = annotations(&mut c, mods);
                cf.fields.push(Field { access: a, name: name.clone(), descriptor, signature, constant, deprecated: false, annotations });
            }
            Member::Method { mods, tparams, ret, name, params, body, annotation_default, .. } => {
                let mut a = access(mods);
                if interface {
                    if a & ACC_PRIVATE == 0 {
                        a |= ACC_PUBLIC;
                    }
                    if !body && a & ACC_STATIC == 0 {
                        a |= ACC_ABSTRACT;
                    }
                }
                if mods.flags & parser::SYNCHRONIZED != 0 {
                    a |= ACC_SYNCHRONIZED;
                }
                if mods.flags & parser::NATIVE != 0 {
                    a |= ACC_NATIVE;
                }
                if params.last().is_some_and(|p| matches!(p.ty, Ty::Repeated(_))) {
                    a |= ACC_VARARGS;
                }
                let n = c.tvars.len();
                c.tvars.extend(tparams.iter());
                let (descriptor, signature) = method_types(&mut c, tparams, params, ret);
                c.tvars.truncate(n);
                let annotations = annotations(&mut c, mods);
                cf.methods.push(Method {
                    access: a,
                    name: name.clone(),
                    descriptor,
                    signature,
                    param_names: Some(params.iter().map(|p| Some(p.name.clone())).collect()),
                    annotations,
                    annotation_default: *annotation_default,
                    ..Default::default()
                });
            }
            Member::Ctor { mods, tparams, params, .. } => {
                ctor = true;
                let n = c.tvars.len();
                c.tvars.extend(tparams.iter());
                let (descriptor, signature) = method_types(&mut c, tparams, params, &Ty::Void);
                let descriptor = with_outer(&inner_outer, descriptor);
                c.tvars.truncate(n);
                // an enum's constructor is private whatever is written (JLS 8.9.2), as javac writes it
                let mut a = if java_enum { ACC_PRIVATE } else { access(mods) & (ACC_PUBLIC | ACC_PROTECTED | ACC_PRIVATE) };
                if params.last().is_some_and(|p| matches!(p.ty, Ty::Repeated(_))) {
                    a |= ACC_VARARGS;
                }
                cf.methods.push(Method { access: a, name: "<init>".into(), descriptor, signature, param_names: Some(params.iter().map(|p| Some(p.name.clone())).collect()), ..Default::default() });
            }
            Member::Type(_) => {}
        }
    }
    // A class without a constructor has the default one, of the class's access (JLS 8.8.9; `makeTemplate`).
    // A record's canonical constructor, unless one of the header's erasure is written (a compact one is skipped,
    // `recordDecl`), of the record's access; an accessor for each component no method without parameters of its name
    // stands for, its result a varargs component's array (`adaptVarargsType`). Neither carries a component's
    // annotations (`recordDecl` makes the accessor without them).
    if record {
        let header: String = decl.header.iter().map(|p| c.erase(&p.ty)).collect();
        let canonical = format!("({})V", header);
        if !cf.methods.iter().any(|m| m.name == "<init>" && m.descriptor == canonical) {
            let (descriptor, signature) = method_types(&mut c, &[], &decl.header, &Ty::Void);
            let mut a = class_access;
            if decl.header.last().is_some_and(|p| matches!(p.ty, Ty::Repeated(_))) {
                a |= ACC_VARARGS;
            }
            cf.methods.push(Method { access: a, name: "<init>".into(), descriptor, signature, param_names: Some(decl.header.iter().map(|p| Some(p.name.clone())).collect()), ..Default::default() });
        }
        for p in &decl.header {
            let written = decl.members.iter().any(|m| matches!(m, Member::Method { name, params, .. } if name == &p.name && params.is_empty()));
            if written {
                continue;
            }
            let (descriptor, signature) = method_types(&mut c, &[], &[], &component_type(&p.ty));
            cf.methods.push(Method { access: ACC_PUBLIC, name: p.name.clone(), descriptor, signature, param_names: Some(Vec::new()), ..Default::default() });
        }
        // javac's `toString`, `hashCode` and `equals` of a record that does not write them (JLS 8.10.3), public final:
        // scalac's answers agree with or without them (java.lang.Record declares the three), so the record is javac's.
        for (name, descriptor, arity) in [("toString", "()Ljava/lang/String;", 0), ("hashCode", "()I", 0), ("equals", "(Ljava/lang/Object;)Z", 1)] {
            let written = decl.members.iter().any(|m| matches!(m, Member::Method { name: n, params, .. } if n == name && params.len() == arity));
            if !written {
                let names = if arity == 1 { vec![Some("o".to_string())] } else { Vec::new() };
                cf.methods.push(Method { access: ACC_PUBLIC | ACC_FINAL, name: name.into(), descriptor: descriptor.into(), param_names: Some(names), ..Default::default() });
            }
        }
    }
    if !ctor && !interface && !record {
        let class_access = if java_enum { ACC_PRIVATE } else { class_access };
        cf.methods.push(Method { access: class_access, name: "<init>".into(), descriptor: with_outer(&inner_outer, "()V".to_string()), param_names: Some(Vec::new()), ..Default::default() });
    }
    conflicting_members(decl, interface, &cf, text, &c.unresolved, &mut c.problems);
    let problems = std::mem::take(&mut c.problems);
    (cf, problems, std::mem::take(&mut c.unresolved))
}

/// Namer's conflicts among a declaration's members (E120, `checkNoDoubleDeclaration`), each at the later one's name:
/// two fields of one name in one scope (a static one is the companion's, so a static and an instance field do not
/// clash), and two methods of one name and one erased signature (their descriptors, `cf.methods` holding one per
/// method in the members' order). In the words teq gives a Scala class's. A method whose signature names a name not
/// resolved (`unresolved`) is in no conflict: its erasure is not known.
fn conflicting_members(decl: &TypeDecl, interface: bool, cf: &ClassFile, text: &str, unresolved: &[String], problems: &mut Vec<Problem>) {
    let line = |at: u32| text[..(at as usize).min(text.len())].matches('\n').count() + 1;
    let place = |statics: bool| match (statics, interface) {
        (true, _) => format!("object {}", decl.name),
        (false, true) => format!("trait {}", decl.name),
        (false, false) => format!("class {}", decl.name),
    };
    let mut descriptors = cf.methods.iter().filter(|m| m.name != "<init>").map(|m| m.descriptor.as_str());
    let mut fields: Vec<(&str, bool, u32, String)> = Vec::new();
    let mut methods: Vec<(&str, &str, bool, u32, String)> = Vec::new();
    for m in &decl.members {
        match m {
            Member::Field { mods, ty, name, at, .. } => {
                let statics = interface || mods.flags & parser::STATIC != 0;
                let kind = if interface || mods.flags & parser::FINAL != 0 { "val" } else { "var" };
                let shown = format!("{} {}: {}", kind, name, scala_text(ty));
                match fields.iter().find(|(n, st, _, _)| n == name && *st == statics) {
                    Some((_, _, first_at, first)) => problems.push(Problem {
                        offset: *at,
                        message: format!("Conflicting definitions:\n{} in {} at line {} and\n{} in {} at line {}", first, place(statics), line(*first_at), shown, place(statics), line(*at)),
                        when_read: false,
                    }),
                    None => fields.push((name, statics, *at, shown)),
                }
            }
            Member::Method { mods, ret, name, params, at, .. } => {
                let Some(descriptor) = descriptors.next() else { continue };
                if unresolved.iter().any(|u| descriptor.contains(&format!("L{};", u))) {
                    continue;
                }
                let statics = mods.flags & parser::STATIC != 0;
                let ps: Vec<String> = params.iter().map(|p| format!("{}: {}", p.name, scala_text(&p.ty))).collect();
                let shown = format!("def {}({}): {}", name, ps.join(", "), scala_text(ret));
                match methods.iter().find(|(n, d, st, _, _)| n == name && *d == descriptor && *st == statics) {
                    Some((_, _, _, first_at, first)) => problems.push(Problem {
                        offset: *at,
                        message: format!(
                            "Conflicting definitions:\n{} in {} at line {} and\n{} in {} at line {}\nhave the same type after erasure.",
                            first,
                            place(statics),
                            line(*first_at),
                            shown,
                            place(statics),
                            line(*at)
                        ),
                        when_read: false,
                    }),
                    None => methods.push((name, descriptor, statics, *at, shown)),
                }
            }
            _ => {}
        }
    }
}

/// A constructor's descriptor with the outer instance first where the class is inner (`inner_outer`).
fn with_outer(inner_outer: &Option<String>, descriptor: String) -> String {
    match inner_outer {
        Some(o) => format!("(L{};{}", o, &descriptor[1..]),
        None => descriptor,
    }
}

/// A record component's type as its field and its accessor have it: a varargs one's array.
fn component_type(t: &Ty) -> Ty {
    match t {
        Ty::Repeated(e) => Ty::Array(e.clone()),
        t => t.clone(),
    }
}

fn method_types(c: &mut Ctx, tparams: &[TParam], params: &[Param], ret: &Ty) -> (String, Option<String>) {
    let mut d = "(".to_string();
    for p in params {
        d.push_str(&c.erase(&p.ty));
    }
    d.push(')');
    d.push_str(&c.erase(ret));
    let generic = !tparams.is_empty() || params.iter().any(|p| c.generic(&p.ty)) || c.generic(ret);
    let sig = generic.then(|| {
        let mut s = c.tparams_sig(tparams);
        s.push('(');
        for p in params {
            s.push_str(&c.sig(&p.ty));
        }
        s.push(')');
        s.push_str(&c.sig(ret));
        s
    });
    (d, sig)
}

/// The annotations of a declaration by their classes' binary names, those it could resolve (an annotation is no
/// dependency: one that names nothing is left out, as the reference drops what it cannot read).
fn annotations(c: &mut Ctx, m: &Mods) -> Vec<String> {
    let mut out = Vec::new();
    for a in &m.annots {
        let found = if a.name.len() == 1 {
            match c.simple(&a.name[0], None) {
                Simple::Found(f) => Some(f),
                _ => None,
            }
        } else {
            c.qualified(&a.name)
        };
        if let Some(f) = found {
            if !f.run_scala {
                out.push(f.binary);
            }
        }
    }
    out
}

/// The record as `TEQ_JAVA_RECORDS=raw` prints it, a line each, the columns by tabs: `RECORD` (its name, access,
/// superclass, interfaces, signature), an `F` a field (access, name, descriptor, signature, constant: `I:7`, `J:7`,
/// `F:` and `D:` its bits in hex, `S:` its UTF-8 bytes in hex), an `M` a method (access, name, descriptor,
/// signature), a `HELD` a problem held for a Scala reader (its offset and message, a line break shown as ` | `),
/// `END`; `-` where there is none.
pub fn raw(cf: &ClassFile, held: &[Problem]) -> String {
    use std::fmt::Write;
    let or = |s: &Option<String>| s.clone().unwrap_or_else(|| "-".to_string());
    let mut out = String::new();
    let _ = writeln!(out, "RECORD\t{}\t{:#06x}\t{}\t{}\t{}", cf.name, cf.access, cf.superclass.as_deref().unwrap_or("-"), cf.interfaces.join(","), or(&cf.signature));
    for f in &cf.fields {
        let constant = match &f.constant {
            Some(ConstValue::Int(v)) => format!("I:{}", v),
            Some(ConstValue::Long(v)) => format!("J:{}", v),
            Some(ConstValue::Float(v)) => format!("F:{:08x}", v.to_bits()),
            Some(ConstValue::Double(v)) => format!("D:{:016x}", v.to_bits()),
            Some(ConstValue::Str(v)) => format!("S:{}", v.bytes().map(|b| format!("{:02x}", b)).collect::<String>()),
            None => "-".to_string(),
        };
        let _ = writeln!(out, "F\t{:#06x}\t{}\t{}\t{}\t{}", f.access, f.name, f.descriptor, or(&f.signature), constant);
    }
    for m in &cf.methods {
        let _ = writeln!(out, "M\t{:#06x}\t{}\t{}\t{}", m.access, m.name, m.descriptor, or(&m.signature));
    }
    for p in held {
        let _ = writeln!(out, "HELD\t{}\t{}", p.offset, p.message.replace('\n', " | "));
    }
    let _ = writeln!(out, "END\t{}", cf.name);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A resolver over a fixed set of classes and packages, by binary name.
    struct Fixed {
        classes: &'static [&'static str],
        packages: &'static [&'static str],
    }

    impl Resolve for Fixed {
        fn class_in(&mut self, pkg: &str, simple: &str) -> Option<Found> {
            let b = if pkg.is_empty() { simple.to_string() } else { format!("{}/{}", pkg, simple) };
            self.classes.contains(&b.as_str()).then_some(Found { binary: b, run_scala: false })
        }
        fn member_of(&mut self, outer: &str, simple: &str) -> Option<Found> {
            let b = format!("{}${}", outer, simple);
            self.classes.contains(&b.as_str()).then_some(Found { binary: b, run_scala: false })
        }
        fn package(&mut self, pkg: &str) -> bool {
            self.packages.contains(&pkg)
        }
        fn parents_of(&mut self, _: &str) -> Vec<String> {
            Vec::new()
        }
        fn member_types(&mut self, _: &str) -> Vec<(String, String, u16)> {
            Vec::new()
        }
        fn is_interface(&mut self, binary: &str) -> bool {
            binary == "java/lang/Comparable"
        }
        fn jdk_rooted(&mut self, pkg: &str) -> bool {
            matches!(pkg.split('/').next(), Some("java" | "jdk"))
        }
        fn member_unseen(&mut self, outer: &str, simple: &str) -> bool {
            outer == "java/util/Map" && simple == "Hidden"
        }
    }

    const CLASSES: &[&str] = &["java/lang/Object", "java/lang/String", "java/lang/Comparable", "java/util/Map", "java/util/Map$Entry", "java/util/Date", "java/sql/Date", "p/Outer", "p/Node", "p/Nope", "p/Unsafe", "q/Unsafe"];
    const PACKAGES: &[&str] = &["java", "java/lang", "java/util", "java/sql", "p", "q"];

    fn record(text: &str) -> (ClassFile, Vec<Problem>, Vec<String>) {
        let unit = parser::parse(text);
        assert!(unit.errors.is_empty(), "{:?}", unit.errors);
        let k = unit.types.iter().position(|t| t.name == "J").unwrap();
        build(&unit, &[], &unit.types[k], text, &mut Fixed { classes: CLASSES, packages: PACKAGES })
    }

    fn at(text: &str, needle: &str, nth: usize) -> u32 {
        text.match_indices(needle).nth(nth).unwrap().0 as u32
    }

    #[test]
    fn a_name_not_found_binds_nothing() {
        let text = "package p;\nimport nope.Node;\nimport java.util.Nope;\nimport static p.Nada.X;\npublic class J extends Outer.Missing implements Node {\n  public Outer.Missing f() { return null; }\n  public void g(Outer.Missing a) {}\n  public void g(Outer.Other b) {}\n  public Nope h() { return null; }\n  public java.util.Map.Absent k() { return null; }\n  public nope.Thing m() { return null; }\n  public java.nope.Thing n() { return null; }\n}\n";
        let (cf, problems, unresolved) = record(text);
        // no class stands for a name not found: not the outer class, not the package's class an import's name would
        // fall through to
        assert_eq!(cf.superclass.as_deref(), Some("Outer/Missing"));
        assert_eq!(cf.interfaces, vec!["Node".to_string()]);
        for m in &cf.methods {
            assert!(!m.descriptor.contains("Lp/"), "{} {}", m.name, m.descriptor);
        }
        let mut want: Vec<&str> = vec!["Outer/Missing", "Node", "Outer/Other", "Nope", "java/util/Map/Absent", "nope/Thing", "java/nope/Thing"];
        let mut got: Vec<&str> = unresolved.iter().map(String::as_str).collect();
        want.sort_unstable();
        got.sort_unstable();
        assert_eq!(got, want);
        // each held for a Scala reader, at the reference's places, in its words; no conflict between the two `g`
        assert!(problems.iter().all(|p| p.when_read), "{:?}", problems.iter().map(|p| &p.message).collect::<Vec<_>>());
        let has = |offset: u32, message: &str| problems.iter().any(|p| p.offset == offset && p.message == message);
        assert!(has(at(text, "Node", 0), "value nope is not a member of <root>"));
        assert!(has(at(text, "X;", 0), "value Nada is not a member of p"));
        assert!(has(at(text, "Missing", 0), "type Missing is not a member of object p.Outer"));
        assert!(has(at(text, "Missing", 1), "type Missing is not a member of object p.Outer"));
        assert!(has(at(text, "Other", 0), "type Other is not a member of object p.Outer"));
        assert!(has(at(text, "Nope h", 0), "Not found: type Nope"));
        assert!(has(at(text, "Absent", 0), "type Absent is not a member of object java.util.Map"));
        assert!(has(at(text, "nope.Thing", 0), "Not found: nope"));
        assert!(has(at(text, "nope.Thing", 1), "value nope is not a member of java"));
        // the import's own fault is told once, at the import: a use of its name adds nothing
        assert!(!problems.iter().any(|p| p.offset == at(text, "Node {", 0)));
        assert!(!problems.iter().any(|p| p.message.starts_with("Conflicting")));
        // an unused import whose qualifier is found and holds no such class is no problem (javac's to refuse)
        assert!(!problems.iter().any(|p| p.offset == at(text, "Nope;", 0)));
    }

    #[test]
    fn an_interface_first_bound_leaves_the_class_bound_empty() {
        let text = "package p;\npublic class J<T extends Comparable<T>, U extends Outer, V extends T> {\n  public <W extends Comparable<W>> W m(T t) { return null; }\n}\n";
        let (cf, problems, _) = record(text);
        assert!(problems.is_empty(), "{:?}", problems.iter().map(|p| &p.message).collect::<Vec<_>>());
        assert_eq!(cf.signature.as_deref(), Some("<T::Ljava/lang/Comparable<TT;>;U:Lp/Outer;V:TT;>Ljava/lang/Object;"));
        let m = cf.methods.iter().find(|m| m.name == "m").unwrap();
        assert_eq!(m.descriptor, "(Ljava/lang/Comparable;)Ljava/lang/Comparable;");
        assert_eq!(m.signature.as_deref(), Some("<W::Ljava/lang/Comparable<TW;>;>(TT;)TW;"));
        // the class's own name as an interface bound, its record not built yet
        let text = "package p;\npublic interface J<T extends J<T>> { }\n";
        let (cf, _, _) = record(text);
        assert_eq!(cf.signature.as_deref(), Some("<T::Lp/J<TT;>;>Ljava/lang/Object;"));
    }

    #[test]
    fn a_jdk_package_the_index_lacks_is_claimed_without_a_problem() {
        // a package-mate `p.Unsafe`, an on-demand `q.*` holding one, and an overload taking `Object`: none takes the name
        let text = "package p;\nimport jdk.internal.misc.Unsafe;\nimport jdk.internal.misc.Unused;\nimport static java.util.Map.Hidden.X;\nimport static java.util.Map.Absent.Y;\nimport q.*;\nimport nope.Node;\npublic class J {\n  public static String f(Unsafe u) { return null; }\n  public static String f(Object o) { return null; }\n}\n";
        let (cf, problems, unresolved) = record(text);
        let shown: Vec<(u32, &str)> = problems.iter().map(|p| (p.offset, p.message.as_str())).collect();
        assert!(problems.iter().all(|p| p.when_read), "{:?}", shown);
        assert!(problems.iter().any(|p| p.offset == at(text, "Unsafe u", 0) && p.message == "Not found: type Unsafe"), "{:?}", shown);
        assert!(problems.iter().any(|p| p.offset == at(text, "Node;", 0) && p.message == "value nope is not a member of <root>"), "{:?}", shown);
        assert!(!problems.iter().any(|p| p.offset == at(text, "Unsafe;", 0) || p.offset == at(text, "Unused;", 0)), "{:?}", shown);
        // a member type the reader does not see is its gap too; one its class has none of is the program's error
        assert!(!problems.iter().any(|p| p.offset == at(text, "X;", 0)), "{:?}", shown);
        assert!(problems.iter().any(|p| p.offset == at(text, "Y;", 0) && p.message == "value Absent is not a member of object java.util.Map"), "{:?}", shown);
        assert!(cf.methods.iter().all(|m| !m.descriptor.contains("Unsafe;") || m.descriptor.contains("LUnsafe;")), "{:?}", cf.methods.iter().map(|m| &m.descriptor).collect::<Vec<_>>());
        assert_eq!(unresolved, vec!["Unsafe".to_string()]);
        assert_eq!(problems.len(), 3, "{:?}", shown);
    }

    #[test]
    fn imports_claim_their_names() {
        let text = "package p;\nimport java.util.Date;\nimport java.util.Date;\nimport java.util.Map.*;\npublic class J {\n  public Date d() { return null; }\n  public Entry<String, String> e() { return null; }\n}\n";
        let (cf, problems, unresolved) = record(text);
        assert!(problems.is_empty(), "{:?}", problems.iter().map(|p| &p.message).collect::<Vec<_>>());
        assert!(unresolved.is_empty());
        let descriptors: Vec<&str> = cf.methods.iter().map(|m| m.descriptor.as_str()).collect();
        assert!(descriptors.contains(&"()Ljava/util/Date;"), "{:?}", descriptors);
        assert!(descriptors.contains(&"()Ljava/util/Map$Entry;"), "{:?}", descriptors);
        let text = "package p;\nimport java.util.Date;\nimport java.sql.Date;\npublic class J {\n  public Date d() { return null; }\n}\n";
        let (_, problems, _) = record(text);
        let use_at = at(text, "Date d", 0);
        assert!(
            problems.iter().any(|p| p.offset == use_at
                && !p.when_read
                && p.message == "reference to Date is ambiguous: it is both imported by name by import java.util.Date and imported by name subsequently by import java.sql.Date"),
            "{:?}",
            problems.iter().map(|p| (p.offset, &p.message)).collect::<Vec<_>>()
        );
    }
}
