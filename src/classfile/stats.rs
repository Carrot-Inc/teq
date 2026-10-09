//! `teq classfile --stats`: over every Java class of an input, how many signatures map onto
//! teq's types and which shapes stand in the way how often, as `teq tasty --stats` does for
//! TASTy. Class files written by a Scala compiler are left out: their signatures come from TASTy.

use super::cli::Input;
use super::shapes::{Arities, Classifier, Found, Mapping, Shape, ALL_SHAPES, SHAPE_COUNT};
use super::sig;
use super::*;
use std::collections::HashMap;
use std::time::Instant;

/// Decodes every signature of a class and returns how many there were; the first malformed one
/// is the error.
pub fn check_signatures(cf: &ClassFile) -> Result<usize, String> {
    sig::class_sig(cf)?;
    for f in &cf.fields {
        sig::field_type(f).map_err(|e| format!("field {}: {}", f.name, e))?;
    }
    for m in &cf.methods {
        sig::member_sig(cf, m).map_err(|e| format!("method {}: {}", m.name, e))?;
    }
    if let Some(components) = &cf.record {
        for c in components {
            sig::parse_type(c.signature.as_deref().unwrap_or(&c.descriptor)).map_err(|e| format!("component {}: {}", c.name, e))?;
        }
    }
    Ok(1 + cf.fields.len() + cf.methods.len())
}

#[derive(Default)]
struct Tally {
    signatures: u64,
    full: u64,
    /// Names a class the arity table does not know without arguments: raw or not, unknown.
    unknown: u64,
    approximated: u64,
    blocked: u64,
    occurrences: [u64; SHAPE_COUNT],
    affected: [u64; SHAPE_COUNT],
}

impl Tally {
    fn record(&mut self, found: &Found) {
        self.signatures += 1;
        for s in ALL_SHAPES {
            let i = s as usize;
            self.occurrences[i] += found.occurrences[i] as u64;
            if found.shapes.has(s) {
                self.affected[i] += 1;
            }
        }
        match found.mapping() {
            Mapping::Blocked => self.blocked += 1,
            Mapping::Approximated => self.approximated += 1,
            Mapping::Maps if !found.unknown_classes.is_empty() => self.unknown += 1,
            Mapping::Maps => self.full += 1,
        }
    }

    fn print(&self, title: &str) {
        let pct = |n: u64| if self.signatures == 0 { 0.0 } else { n as f64 * 100.0 / self.signatures as f64 };
        println!("{}: {} signatures", title, self.signatures);
        println!("  {:>8} {:5.1}%  map fully", self.full, pct(self.full));
        println!("  {:>8} {:5.1}%  map fully if the classes they name outside the input take no type arguments", self.unknown, pct(self.unknown));
        println!("  {:>8} {:5.1}%  load with an approximation (shapes marked ~)", self.approximated, pct(self.approximated));
        println!("  {:>8} {:5.1}%  blocked by an unsupported shape (marked !)", self.blocked, pct(self.blocked));
        let mut order: Vec<Shape> = ALL_SHAPES.iter().copied().filter(|&s| self.affected[s as usize] > 0).collect();
        order.sort_by_key(|&s| std::cmp::Reverse(self.affected[s as usize]));
        if !order.is_empty() {
            println!("  {:>10} {:>11}   shape", "signatures", "occurrences");
        }
        for s in order {
            let mark = match s.mapping() {
                Mapping::Maps => " ",
                Mapping::Approximated => "~",
                Mapping::Blocked => "!",
            };
            println!("  {:>10} {:>11} {} {}", self.affected[s as usize], self.occurrences[s as usize], mark, s.describe());
        }
    }
}

#[derive(Default)]
struct Counts {
    files: u64,
    scala: u64,
    classes: u64,
    interfaces: u64,
    enums: u64,
    records: u64,
    annotations: u64,
    modules: u64,
    nested_static: u64,
    nested_inner: u64,
    local_or_anonymous: u64,
    with_signature: u64,
    sealed: u64,
    fields: u64,
    static_fields: u64,
    methods: u64,
    static_methods: u64,
    ctors: u64,
    default_methods: u64,
    varargs: u64,
    synthetic: u64,
    deprecated: u64,
    members_with_signature: u64,
}

#[derive(Default)]
struct Stats {
    counts: Counts,
    headers: Tally,
    members: Tally,
    unknown_classes: HashMap<String, u64>,
    /// Blocked member signatures per package.
    packages: HashMap<String, (u64, u64)>,
}

fn visit(cf: &ClassFile, c: &Classifier, stats: &mut Stats) {
    let n = &mut stats.counts;
    n.files += 1;
    if cf.scala {
        n.scala += 1;
        return;
    }
    let flags = cf.own_inner_entry().map_or(cf.access, |i| i.access);
    if flags & ACC_MODULE != 0 {
        n.modules += 1;
    } else if flags & ACC_ANNOTATION != 0 {
        n.annotations += 1;
    } else if flags & ACC_INTERFACE != 0 {
        n.interfaces += 1;
    } else if flags & ACC_ENUM != 0 {
        n.enums += 1;
    } else if cf.record.is_some() {
        n.records += 1;
    } else {
        n.classes += 1;
    }
    if let Some(own) = cf.own_inner_entry() {
        if own.outer.is_none() || own.name.is_none() {
            n.local_or_anonymous += 1;
        } else if own.access & ACC_STATIC != 0 {
            n.nested_static += 1;
        } else {
            n.nested_inner += 1;
        }
    }
    if cf.signature.is_some() {
        n.with_signature += 1;
    }
    if !cf.permitted.is_empty() {
        n.sealed += 1;
    }
    let package = cf.name.rfind('/').map_or(String::new(), |at| cf.name[..at].replace('/', "."));
    if let Ok(header) = sig::class_sig(cf) {
        let found = c.class_header(&header);
        note_unknown(stats, &found);
        stats.headers.record(&found);
    }
    for f in &cf.fields {
        let n = &mut stats.counts;
        n.fields += 1;
        if f.access & ACC_STATIC != 0 {
            n.static_fields += 1;
        }
        if f.signature.is_some() {
            n.members_with_signature += 1;
        }
        if f.deprecated {
            n.deprecated += 1;
        }
        if f.access & ACC_SYNTHETIC != 0 {
            n.synthetic += 1;
            continue;
        }
        if let Ok(t) = sig::field_type(f) {
            let mut found = Found::default();
            c.walk(&t, &mut found);
            record_member(stats, &package, &found);
        }
    }
    for m in &cf.methods {
        if m.name == "<clinit>" {
            continue;
        }
        let n = &mut stats.counts;
        if m.name == "<init>" {
            n.ctors += 1;
        } else {
            n.methods += 1;
            if m.access & ACC_STATIC != 0 {
                n.static_methods += 1;
            } else if cf.is_interface() && m.access & ACC_ABSTRACT == 0 {
                n.default_methods += 1;
            }
        }
        if m.access & ACC_VARARGS != 0 {
            n.varargs += 1;
        }
        if m.signature.is_some() {
            n.members_with_signature += 1;
        }
        if m.deprecated {
            n.deprecated += 1;
        }
        if m.access & ACC_SYNTHETIC != 0 {
            n.synthetic += 1;
            continue;
        }
        if let Ok(sig) = sig::member_sig(cf, m) {
            let found = c.method(&sig, m.access & ACC_VARARGS != 0);
            record_member(stats, &package, &found);
        }
    }
}

fn note_unknown(stats: &mut Stats, found: &Found) {
    for u in &found.unknown_classes {
        *stats.unknown_classes.entry(u.clone()).or_default() += 1;
    }
}

fn record_member(stats: &mut Stats, package: &str, found: &Found) {
    note_unknown(stats, found);
    stats.members.record(found);
    let slot = stats.packages.entry(package.to_string()).or_default();
    slot.0 += 1;
    slot.1 += (found.mapping() == Mapping::Blocked) as u64;
}

fn arities_of(input: &Input, filter: Option<&str>, arities: &mut Arities) -> Result<usize, String> {
    let items = input.java_items(filter)?;
    for item in &items {
        let data = input.read(item)?;
        let Ok(cf) = ClassFile::parse(&data) else { continue };
        let arity = match &cf.signature {
            Some(s) => sig::parse_class(s).map_or(0, |c| c.tparams.len()),
            None => 0,
        };
        arities.of.insert(cf.name, arity as u16);
    }
    Ok(items.len())
}

pub fn run(input: &Input, filter: Option<&str>) {
    let start = Instant::now();
    let mut arities = Arities::default();
    let mut jdk_note = String::new();
    arities_of(input, None, &mut arities).unwrap_or_else(|e| fail(e));
    if !matches!(input, Input::CtSym { .. }) {
        match Input::jdk(None) {
            Ok(jdk) => {
                let n = arities_of(&jdk, None, &mut arities).unwrap_or_else(|e| fail(e));
                jdk_note = format!("; the arities of {} JDK classes ({}) tell raw types apart", n, jdk.describe());
            }
            Err(_) => jdk_note = "; no JDK found, so references to its generic classes count as unknown".to_string(),
        }
    }
    let arities_done = start.elapsed();
    let c = Classifier { arities: &arities };
    let mut stats = Stats::default();
    let items = input.java_items(filter).unwrap_or_else(|e| fail(e));
    let mut bytes = 0usize;
    for item in &items {
        let data = input.read(item).unwrap_or_else(|e| fail(e));
        bytes += data.len();
        match ClassFile::parse(&data) {
            Ok(cf) => visit(&cf, &c, &mut stats),
            Err(e) => println!("{}: {}", item.entry, e),
        }
    }
    let elapsed = start.elapsed();
    let n = &stats.counts;
    println!("{}{}", input.describe(), filter.map_or(String::new(), |f| format!(", filter {}", f)));
    println!(
        "  {} class files ({} bytes) read and classified in {:?} ({:?} of it building the arity table of {} classes{})",
        n.files,
        bytes,
        elapsed,
        arities_done,
        arities.of.len(),
        jdk_note
    );
    println!(
        "  {} class files with a TASTy sibling left out by name; {} carry a Scala attribute anyway; {} Java: {} classes, {} interfaces, {} enums, {} records, {} annotations, {} module descriptors",
        input.items(filter).map_or(0, |all| all.len() as u64) - n.files,
        n.scala,
        n.files - n.scala,
        n.classes,
        n.interfaces,
        n.enums,
        n.records,
        n.annotations,
        n.modules
    );
    println!(
        "  nested: {} static, {} inner (an outer instance), {} local or anonymous; {} with a Signature attribute (generic or with generic parents); {} sealed",
        n.nested_static, n.nested_inner, n.local_or_anonymous, n.with_signature, n.sealed
    );
    println!(
        "  members: {} fields ({} static), {} methods ({} static, {} default), {} constructors; {} varargs, {} deprecated, {} with a Signature attribute; {} synthetic or bridge members left out",
        n.fields, n.static_fields, n.methods, n.static_methods, n.default_methods, n.ctors, n.varargs, n.deprecated, n.members_with_signature, n.synthetic
    );
    stats.members.print("  member signatures (fields, methods, constructors)");
    print_packages(&stats.packages);
    stats.headers.print("  class headers (type parameters, superclass, interfaces)");
    let mut unknown: Vec<(&String, &u64)> = stats.unknown_classes.iter().collect();
    unknown.sort_by(|a, b| b.1.cmp(a.1).then(a.0.cmp(b.0)));
    if !unknown.is_empty() {
        let total: u64 = unknown.iter().map(|u| *u.1).sum();
        let shown: Vec<String> = unknown.iter().take(10).map(|(t, n)| format!("{} {}", t, n)).collect();
        println!("  classes named outside the input: {} references to {}; most frequent: {}", total, unknown.len(), shown.join(", "));
    }
}

fn print_packages(packages: &HashMap<String, (u64, u64)>) {
    let mut rows: Vec<(&String, &(u64, u64))> = packages.iter().filter(|(_, v)| v.1 > 0).collect();
    rows.sort_by(|a, b| b.1 .1.cmp(&a.1 .1).then(a.0.cmp(b.0)));
    if rows.is_empty() {
        return;
    }
    let blocked: u64 = rows.iter().map(|r| r.1 .1).sum();
    println!("  blocked member signatures by package ({} in {} packages):", blocked, rows.len());
    for (name, (all, b)) in rows.iter().take(12) {
        println!("  {:>10} of {:>6}  {}", b, all, name);
    }
}

fn fail(msg: String) -> ! {
    eprintln!("{}", msg);
    std::process::exit(1);
}
