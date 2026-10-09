//! `teq tasty --stats`: over every signature of the given jars, how many map onto teq's types and
//! which shapes stand in the way how often. The numbers size the work on the type system.

use super::shapes::{Classifier, Found, Shape, ALL_SHAPES, SHAPE_COUNT};
use super::tags::*;
use super::terms::*;
use super::tree::{index_top_level, Clause, Decoder, DefSig, Entry, TType};
use super::TastyFile;
use std::collections::HashMap;
use std::time::Instant;

#[derive(Default)]
struct Tally {
    signatures: u64,
    /// No shape outside teq's types, and no type member on a static path.
    full: u64,
    /// As `full` but for type members on static paths (`Outer.T`), which map when `T` is a class
    /// or an alias and not when it is abstract; the name alone does not tell.
    full_if_static_members_resolve: u64,
    /// Only shapes that teq reads as something weaker.
    approximated: u64,
    blocked: u64,
    occurrences: [u64; SHAPE_COUNT],
    affected: [u64; SHAPE_COUNT],
    /// What the references of the unresolved kind name, with how often.
    static_targets: std::collections::HashMap<String, u64>,
    member_targets: std::collections::HashMap<(u8, String), u64>,
}

impl Tally {
    fn record(&mut self, found: &Found) {
        self.signatures += 1;
        for t in &found.static_targets {
            *self.static_targets.entry(t.clone()).or_default() += 1;
        }
        for (shape, t) in &found.member_targets {
            *self.member_targets.entry((*shape as u8, t.clone())).or_default() += 1;
        }
        for s in ALL_SHAPES {
            let i = s as usize;
            self.occurrences[i] += found.occurrences[i] as u64;
            if found.shapes.has(s) {
                self.affected[i] += 1;
            }
        }
        let blocked = ALL_SHAPES.iter().any(|&s| found.shapes.has(s) && !s.approximated() && !s.supported());
        let approximated = ALL_SHAPES.iter().any(|&s| found.shapes.has(s) && s.approximated());
        if blocked {
            self.blocked += 1;
        } else if approximated {
            self.approximated += 1;
        } else if found.static_members > 0 {
            self.full_if_static_members_resolve += 1;
        } else {
            self.full += 1;
        }
    }

    fn add(&mut self, other: &Tally) {
        self.signatures += other.signatures;
        self.full += other.full;
        self.full_if_static_members_resolve += other.full_if_static_members_resolve;
        self.approximated += other.approximated;
        self.blocked += other.blocked;
        for i in 0..SHAPE_COUNT {
            self.occurrences[i] += other.occurrences[i];
            self.affected[i] += other.affected[i];
        }
        for (t, n) in &other.static_targets {
            *self.static_targets.entry(t.clone()).or_default() += n;
        }
        for (k, n) in &other.member_targets {
            *self.member_targets.entry(k.clone()).or_default() += n;
        }
    }

    fn print(&self, title: &str) {
        let pct = |n: u64| if self.signatures == 0 { 0.0 } else { n as f64 * 100.0 / self.signatures as f64 };
        println!("{}: {} signatures", title, self.signatures);
        println!("  {:>8} {:5.1}%  map fully", self.full, pct(self.full));
        println!(
            "  {:>8} {:5.1}%  map fully if the type members they name on static paths are classes or aliases",
            self.full_if_static_members_resolve,
            pct(self.full_if_static_members_resolve)
        );
        println!("  {:>8} {:5.1}%  load with an approximation (shapes marked ~)", self.approximated, pct(self.approximated));
        println!("  {:>8} {:5.1}%  blocked by an unsupported shape", self.blocked, pct(self.blocked));
        let mut order: Vec<Shape> = ALL_SHAPES.iter().copied().filter(|&s| self.affected[s as usize] > 0).collect();
        order.sort_by_key(|&s| std::cmp::Reverse(self.affected[s as usize]));
        if !order.is_empty() {
            println!("  {:>10} {:>11}   shape", "signatures", "occurrences");
        }
        for s in order {
            let mark = if s.approximated() { "~" } else { " " };
            println!("  {:>10} {:>11} {} {}", self.affected[s as usize], self.occurrences[s as usize], mark, s.describe());
        }
        for shape in [Shape::TypeMember, Shape::PathDependent] {
            let mut named: Vec<(&String, u64)> =
                self.member_targets.iter().filter(|((s, _), _)| *s == shape as u8).map(|((_, t), n)| (t, *n)).collect();
            named.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(b.0)));
            if !named.is_empty() {
                let shown: Vec<String> = named.iter().take(12).map(|(t, n)| format!("{} {}", t, n)).collect();
                println!("  {} names {} targets; most frequent: {}", shape.describe(), named.len(), shown.join(", "));
            }
        }
        let mut targets: Vec<(&String, &u64)> = self.static_targets.iter().collect();
        targets.sort_by(|a, b| b.1.cmp(a.1).then(a.0.cmp(b.0)));
        if !targets.is_empty() {
            let total: u64 = targets.iter().map(|t| *t.1).sum();
            let shown: Vec<String> = targets.iter().take(10).map(|(t, n)| format!("{} {}", t, n)).collect();
            println!("  type members named on static paths: {} references to {} targets; most frequent: {}", total, targets.len(), shown.join(", "));
        }
    }
}

#[derive(Default)]
struct JarStats {
    files: u64,
    classes: u64,
    headers: Tally,
    members: Tally,
    /// Member signatures per package: (signatures, blocked).
    packages: std::collections::HashMap<String, (u64, u64)>,
}

fn def_found(c: &Classifier, sig: &DefSig) -> Found {
    let mut found = Found::default();
    for clause in &sig.clauses {
        match clause {
            Clause::Types(ps) => ps.iter().for_each(|p| c.walk(&p.info, &mut found)),
            Clause::Terms(ps) => ps.iter().for_each(|p| c.top(&p.ty, &mut found)),
        }
    }
    c.top(&sig.ret, &mut found);
    found
}

fn visit_class(file: &TastyFile, decoder: &mut Decoder, entry: &Entry, package: &str, stats: &mut JarStats) {
    let sig = decoder.class_sig(entry.addr);
    let lookup = Decoder::new(file);
    let c = Classifier { file, decoder: &lookup };
    stats.classes += 1;
    let mut header = Found::default();
    sig.tparams.iter().for_each(|p| c.walk(&p.info, &mut header));
    sig.parents.iter().for_each(|p| c.walk(p, &mut header));
    if let Some((_, t)) = sig.self_type.as_ref().filter(|_| !sig.mods.flags.has(OBJECT)) {
        c.walk(t, &mut header);
    }
    if let Some(ctor) = &sig.ctor {
        let ctor_found = def_found(&c, ctor);
        for s in ALL_SHAPES {
            for _ in 0..ctor_found.occurrences[s as usize] {
                header.shapes.add(s);
                header.occurrences[s as usize] += 1;
            }
        }
        header.static_members += ctor_found.static_members;
        header.static_targets.extend(ctor_found.static_targets);
        header.member_targets.extend(ctor_found.member_targets);
    }
    stats.headers.record(&header);
    for m in &sig.index.members {
        match m.tag {
            TYPEDEF if m.is_class => visit_class(file, decoder, m, package, stats),
            TYPEDEF => {
                let t = decoder.type_def_sig(m.addr);
                let mut found = Found::default();
                c.walk(&t.rhs, &mut found);
                if let Some(b) = &t.opaque_bounds {
                    c.walk(b, &mut found);
                }
                record_member(stats, package, &found);
            }
            VALDEF if m.flags.has(OBJECT) => {}
            _ => {
                let d = decoder.def_sig(m.addr);
                record_member(stats, package, &def_found(&c, &d));
            }
        }
    }
}

fn record_member(stats: &mut JarStats, package: &str, found: &Found) {
    stats.members.record(found);
    let blocked = ALL_SHAPES.iter().any(|&s| found.shapes.has(s) && !s.approximated() && !s.supported());
    let slot = stats.packages.entry(package.to_string()).or_default();
    slot.0 += 1;
    slot.1 += blocked as u64;
}

fn print_packages(packages: &std::collections::HashMap<String, (u64, u64)>) {
    let mut rows: Vec<(&String, &(u64, u64))> = packages.iter().filter(|(_, v)| v.1 > 0).collect();
    rows.sort_by(|a, b| b.1 .1.cmp(&a.1 .1).then(a.0.cmp(b.0)));
    let blocked: u64 = rows.iter().map(|r| r.1 .1).sum();
    println!("  blocked member signatures by package ({} in {} packages):", blocked, rows.len());
    for (name, (all, b)) in rows.iter().take(12) {
        println!("  {:>10} of {:>6}  {}", b, all, name);
    }
}

pub fn run(paths: &[&str]) {
    let mut total = JarStats::default();
    for &path in paths {
        let start = Instant::now();
        let mut stats = JarStats::default();
        super::cli::each_tasty(path, None, |_, bytes| {
            let Ok(file) = TastyFile::parse(bytes) else { return };
            stats.files += 1;
            let mut decoder = Decoder::new(&file);
            for top in index_top_level(&file) {
                if top.entry.tag == TYPEDEF && top.entry.is_class {
                    let package = file.name(top.package);
                    visit_class(&file, &mut decoder, &top.entry, &package, &mut stats);
                }
            }
        });
        let elapsed = start.elapsed();
        let signatures = stats.headers.signatures + stats.members.signatures;
        println!("{}", path);
        println!(
            "  {} TASTy files, {} classes, {} signatures read and decoded in {:?} ({:.0} signatures per second)",
            stats.files,
            stats.classes,
            signatures,
            elapsed,
            signatures as f64 / elapsed.as_secs_f64()
        );
        stats.members.print("  member signatures");
        print_packages(&stats.packages);
        stats.headers.print("  class headers (type parameters, parents, self type, constructor)");
        println!();
        total.files += stats.files;
        total.classes += stats.classes;
        total.members.add(&stats.members);
        total.headers.add(&stats.headers);
    }
    if paths.len() > 1 {
        println!("all {} jars: {} TASTy files, {} classes", paths.len(), total.files, total.classes);
        total.members.print("  member signatures");
        total.headers.print("  class headers");
    }
}

// ---- terms: `teq tasty --term-stats` ----

/// Names of `scala.compiletime` and `scala.quoted` that an inline expander has to evaluate
/// itself: their bodies in the jar are placeholders.
const INTRINSICS: &[&str] = &[
    "summonInline",
    "summonFrom",
    "summonAll",
    "erasedValue",
    "constValue",
    "constValueOpt",
    "constValueTuple",
    "error",
    "codeOf",
    "requireConst",
    "typeChecks",
    "typeCheckErrors",
    "uninitialized",
];

#[derive(Default)]
struct Census {
    bodies: u64,
    rhs_bytes: u64,
    tags: HashMap<&'static str, u64>,
    /// (a) inline method bodies: tags, and per method what it needs.
    inline_bodies: u64,
    inline_tags: HashMap<&'static str, u64>,
    inline_defs: Vec<(String, Vec<String>)>,
    /// (b) bodies with quotes or splices, by package, and the `scala.quoted` members they call.
    macro_bodies: HashMap<String, u64>,
    quote_bytes: u64,
    quotes_api: HashMap<String, u64>,
    quotes_api_all: HashMap<String, u64>,
    /// Calls in the other bodies of `inline` defs of the same jar, by owner and name.
    inline_calls: HashMap<String, u64>,
    /// (c) the rest: shapes outside the typed IR, by occurrence and by bodies affected, and the
    /// Java members called.
    rest_bodies: u64,
    beyond: HashMap<&'static str, (u64, u64, String)>,
    java_members: HashMap<String, u64>,
}

impl Census {
    fn add(&mut self, other: &Census) {
        self.bodies += other.bodies;
        self.rhs_bytes += other.rhs_bytes;
        self.inline_bodies += other.inline_bodies;
        self.rest_bodies += other.rest_bodies;
        self.quote_bytes += other.quote_bytes;
        for (k, n) in &other.inline_calls {
            *self.inline_calls.entry(k.clone()).or_default() += n;
        }
        for (k, n) in &other.tags {
            *self.tags.entry(k).or_default() += n;
        }
        for (k, n) in &other.inline_tags {
            *self.inline_tags.entry(k).or_default() += n;
        }
        self.inline_defs.extend(other.inline_defs.iter().cloned());
        for (k, n) in &other.macro_bodies {
            *self.macro_bodies.entry(k.clone()).or_default() += n;
        }
        for (k, n) in &other.quotes_api {
            *self.quotes_api.entry(k.clone()).or_default() += n;
        }
        for (k, n) in &other.quotes_api_all {
            *self.quotes_api_all.entry(k.clone()).or_default() += n;
        }
        for (k, (a, b, example)) in &other.beyond {
            let e = self.beyond.entry(k).or_insert_with(|| (0, 0, example.clone()));
            e.0 += a;
            e.1 += b;
        }
        for (k, n) in &other.java_members {
            *self.java_members.entry(k.clone()).or_default() += n;
        }
    }
}

/// What one body is made of, gathered by a walk.
#[derive(Default)]
struct BodyFacts {
    tags: HashMap<&'static str, u64>,
    beyond: HashMap<&'static str, u64>,
    needs: Vec<String>,
    has_quotes: bool,
    quote_bytes: u64,
    quotes_api: Vec<String>,
    java_members: Vec<String>,
    inline_calls: Vec<String>,
}

fn short_name(q: &str) -> &str {
    q.rsplit('.').next().unwrap_or(q)
}

fn body_facts(file: &TastyFile, d: &Decoder, inline_names: &std::collections::HashSet<String>, body: &Term, method: Option<super::tree::Addr>, inline: bool) -> BodyFacts {
    let mut facts = BodyFacts::default();
    let mut f = |t: &Term, scope: &Scope| {
        *facts.tags.entry(tag_name(t)).or_default() += 1;
        let mut beyond = |what: &'static str| *facts.beyond.entry(what).or_default() += 1;
        match &t.kind {
            TermKind::SelectOuter { .. } => beyond("SELECTouter"),
            TermKind::Return { from, .. } => {
                if scope.method != Some(*from) {
                    beyond("RETURN from a nested method or lambda");
                } else if scope.method != method {
                    beyond("RETURN inside a local def");
                }
            }
            TermKind::While(..) => beyond("WHILE (in the IR)"),
            TermKind::Try { cases, finalizer, .. } => {
                beyond(match (cases.is_empty(), finalizer.is_some()) {
                    (false, false) => "TRY catch",
                    (false, true) => "TRY catch finally",
                    (true, true) => "TRY finally",
                    (true, false) => "TRY alone",
                });
                if cases.iter().any(|c| c.guard.is_some()) {
                    beyond("TRY case with a guard");
                }
            }
            TermKind::Super(this, mixin) => {
                if mixin.is_some() {
                    beyond("SUPER[mixin]");
                } else if matches!(&this.kind, TermKind::QualThis(_)) {
                    beyond("C.super with a qualifier");
                }
            }
            TermKind::Typed(e, ..) if matches!(&e.kind, TermKind::Repeated(..)) => beyond("TYPED repeated argument (xs*)"),
            TermKind::Const(super::tree::Const::Class(_)) => beyond("CLASSconst"),
            TermKind::NamedArg(..) => beyond("NAMEDARG"),
            TermKind::ApplySigPoly(..) => beyond("APPLYsigpoly"),
            TermKind::Elided(_) => beyond("ELIDED"),
            TermKind::Lambda(_, Some(_)) => beyond("LAMBDA with a SAM type"),
            TermKind::Match { kind: MatchKind::Sub, .. } => beyond("MATCH sub"),
            TermKind::Inlined { .. } if !inline => beyond("INLINED (expanded already)"),
            TermKind::Block(stats, _) => {
                for s in stats {
                    match s {
                        Stat::Class(c) if file.name(c.name).starts_with("$anon") => beyond("anonymous class"),
                        Stat::Class(_) => beyond("local class"),
                        Stat::Type(_) => beyond("local type alias"),
                        Stat::Val(sig, _) if sig.mods.flags.has(LAZY) => beyond("local lazy val"),
                        Stat::Def(sig, _) if sig.mods.flags.has(INLINE) => beyond("local inline def"),
                        _ => {}
                    }
                }
            }
            TermKind::Unapply { implicits, .. } if !implicits.is_empty() => beyond("UNAPPLY with implicit arguments"),
            TermKind::Quote { span, .. } | TermKind::QuotePattern { span, .. } => {
                facts.has_quotes = true;
                if !scope.in_quote {
                    facts.quote_bytes += (span.end - span.start) as u64;
                }
            }
            TermKind::Splice { .. } | TermKind::SplicePattern { .. } => facts.has_quotes = true,
            _ => {}
        }
        let (owner, name) = match &t.kind {
            TermKind::SelectIn(_, n, owner, _) => (Some(owner), Some(*n)),
            TermKind::Select(_, n) => (None, Some(*n)),
            TermKind::Path(TType::TermRef(prefix, n)) if !matches!(&**prefix, TType::Package(_)) => (Some(&**prefix), Some(*n)),
            TermKind::Path(TType::TermRef(_, n)) => (None, Some(*n)),
            _ => (None, None),
        };
        if let Some(n) = name {
            let member = file.name(file.source_name(n));
            if INTRINSICS.contains(&member.as_str()) && inline {
                facts.needs.push(member.clone());
            }
            if matches!(member.as_str(), "asInstanceOf" | "isInstanceOf" | "synchronized" | "getClass" | "##" | "eq" | "ne") {
                *facts.beyond.entry(match member.as_str() {
                    "asInstanceOf" => "asInstanceOf",
                    "isInstanceOf" => "isInstanceOf",
                    "synchronized" => "synchronized",
                    "getClass" => "getClass",
                    "##" => "##",
                    _ => "eq / ne",
                })
                .or_default() += 1;
            }
            if let Some(owner) = owner {
                let q = qualified_name(file, d, owner);
                let qualified = format!("{}.{}", q, member);
                if inline_names.contains(&qualified) {
                    facts.inline_calls.push(qualified);
                }
                if q.starts_with("java.") {
                    facts.java_members.push(format!("{}.{}", q, member));
                }
                // Inside a quote a call is generated code; outside one the interpreter runs it.
                if q.starts_with("scala.quoted") && !scope.in_quote {
                    facts.quotes_api.push(format!("{}.{}", short_name(&q), member));
                }
            }
        }
        match &t.kind {
            TermKind::Match { kind: MatchKind::Inline | MatchKind::Implicit, .. } if inline => facts.needs.push("inline match".to_string()),
            TermKind::If { inline: true, .. } if inline => facts.needs.push("inline if".to_string()),
            TermKind::Splice { .. } if inline => facts.needs.push("${ }".to_string()),
            TermKind::Quote { .. } if inline => facts.needs.push("'{ }".to_string()),
            _ => {}
        }
    };
    Walk { f: &mut f }.term(body, Scope { method, in_inline: inline, in_quote: false });
    facts.needs.sort();
    facts.needs.dedup();
    facts
}

fn record_body(c: &mut Census, file: &TastyFile, d: &Decoder, inline_names: &std::collections::HashSet<String>, package: &str, owner: &str, sig: Option<&DefSig>, body: &Term) {
    c.bodies += 1;
    let inline = sig.map_or(false, |s| s.mods.flags.has(INLINE));
    let facts = body_facts(file, d, inline_names, body, sig.map(|s| s.addr), inline);
    for (k, n) in &facts.tags {
        *c.tags.entry(k).or_default() += n;
    }
    for m in &facts.quotes_api {
        *c.quotes_api_all.entry(m.clone()).or_default() += 1;
    }
    if inline {
        c.inline_bodies += 1;
        for (k, n) in &facts.tags {
            *c.inline_tags.entry(k).or_default() += n;
        }
        c.inline_defs.push((owner.to_string(), facts.needs));
    } else if facts.has_quotes {
        *c.macro_bodies.entry(package.to_string()).or_default() += 1;
        c.quote_bytes += facts.quote_bytes;
        for m in &facts.quotes_api {
            *c.quotes_api.entry(m.clone()).or_default() += 1;
        }
    } else {
        c.rest_bodies += 1;
        for (k, n) in &facts.beyond {
            let e = c.beyond.entry(k).or_insert_with(|| (0, 0, owner.to_string()));
            e.0 += n;
            e.1 += 1;
        }
        for m in &facts.java_members {
            *c.java_members.entry(m.clone()).or_default() += 1;
        }
        for m in &facts.inline_calls {
            *c.inline_calls.entry(m.clone()).or_default() += 1;
        }
    }
}

fn census_stat(c: &mut Census, file: &TastyFile, d: &Decoder, td: &mut TermDecoder, inline_names: &std::collections::HashSet<String>, package: &str, s: &Stat) {
    let mut all = Vec::new();
    bodies(file, s, package, &mut all);
    for (owner, sig, body) in all {
        if let Some(b) = sig.and_then(|s| s.body) {
            c.rhs_bytes += td.tree_len(b) as u64;
        }
        record_body(c, file, d, inline_names, package, &owner, sig, body);
    }
}

/// The `inline` defs a statement declares, at any class depth, as `owner.name`.
fn inline_names(file: &TastyFile, s: &Stat, owner: &str, out: &mut std::collections::HashSet<String>) {
    match s {
        Stat::Def(sig, _) if sig.mods.flags.has(INLINE) => {
            out.insert(format!("{}.{}", owner, file.name(sig.name)));
        }
        Stat::Class(c) => {
            let name = format!("{}.{}", owner, file.name(file.source_name(c.name)));
            c.template.stats.iter().for_each(|s| inline_names(file, s, &name, out))
        }
        _ => {}
    }
}

fn top_counts<'k, K: std::fmt::Display + Ord>(m: &'k HashMap<K, u64>, n: usize) -> String {
    let mut rows: Vec<(&K, &u64)> = m.iter().collect();
    rows.sort_by(|a, b| b.1.cmp(a.1).then(a.0.cmp(b.0)));
    rows.iter().take(n).map(|(k, v)| format!("{} {}", k, v)).collect::<Vec<_>>().join(", ")
}

fn print_census(c: &Census, title: &str) {
    println!("{}: {} bodies, {} bytes of right-hand sides", title, c.bodies, c.rhs_bytes);
    let mut tags: Vec<(&&str, &u64)> = c.tags.iter().collect();
    tags.sort_by(|a, b| b.1.cmp(a.1).then(a.0.cmp(b.0)));
    println!("  term tags by occurrence:");
    for (k, n) in &tags {
        println!("  {:>9}  {}", n, k);
    }
    println!("  (a) inline method bodies: {}; tags: {}", c.inline_bodies, top_counts(&c.inline_tags, 40));
    let mut needs: HashMap<&str, u64> = HashMap::new();
    for (_, ns) in &c.inline_defs {
        for n in ns {
            *needs.entry(n).or_default() += 1;
        }
    }
    println!("      constructs needed, by inline defs: {}", top_counts(&needs, 20));
    let mut defs: Vec<&(String, Vec<String>)> = c.inline_defs.iter().filter(|(_, ns)| !ns.is_empty()).collect();
    defs.sort();
    println!("      inline defs that need one ({} of {}):", defs.len(), c.inline_defs.len());
    for (name, ns) in defs {
        println!("        {}: {}", name, ns.join(", "));
    }
    let macro_total: u64 = c.macro_bodies.values().sum();
    println!("  (b) bodies with quotes or splices: {} ({} bytes inside quotes); by package: {}", macro_total, c.quote_bytes, top_counts(&c.macro_bodies, 20));
    let api_total: u64 = c.quotes_api.values().sum();
    println!("      scala.quoted members they call: {} calls of {} members; most frequent: {}", api_total, c.quotes_api.len(), top_counts(&c.quotes_api, 40));
    let api_all: u64 = c.quotes_api_all.values().sum();
    println!("      scala.quoted members called by any body: {} calls of {} members; most frequent: {}", api_all, c.quotes_api_all.len(), top_counts(&c.quotes_api_all, 60));
    println!("  (c) other bodies: {}; shapes outside the typed IR (occurrences, bodies, one example):", c.rest_bodies);
    let mut rows: Vec<(&&str, &(u64, u64, String))> = c.beyond.iter().collect();
    rows.sort_by(|a, b| b.1 .1.cmp(&a.1 .1).then(a.0.cmp(b.0)));
    for (k, (occ, bodies, example)) in rows {
        println!("  {:>9} {:>7}  {}  ({})", occ, bodies, k, example);
    }
    let inline_total: u64 = c.inline_calls.values().sum();
    println!("      calls of inline defs of the same jar: {} calls of {} defs; most frequent: {}", inline_total, c.inline_calls.len(), top_counts(&c.inline_calls, 30));
    let java_total: u64 = c.java_members.values().sum();
    let shown = if std::env::var_os("TEQ_CENSUS_ALL").is_some() { usize::MAX } else { 40 };
    println!("      Java members called: {} calls of {} members; most frequent: {}", java_total, c.java_members.len(), top_counts(&c.java_members, shown));
}

pub fn run_terms(paths: &[&str]) {
    let mut total = Census::default();
    for &path in paths {
        let start = Instant::now();
        let mut c = Census::default();
        let mut files = 0u64;
        let mut names = std::collections::HashSet::new();
        super::cli::each_tasty(path, None, |_, bytes| {
            let Ok(file) = TastyFile::parse(bytes) else { return };
            let mut decoder = Decoder::new(&file);
            let mut td = TermDecoder::new(&mut decoder);
            for top in index_top_level(&file) {
                let package = file.name(top.package);
                inline_names(&file, &td.stat_at(top.entry.addr), &package, &mut names);
            }
        });
        super::cli::each_tasty(path, None, |_, bytes| {
            let Ok(file) = TastyFile::parse(bytes) else { return };
            files += 1;
            let lookup = Decoder::new(&file);
            let mut decoder = Decoder::new(&file);
            let mut td = TermDecoder::new(&mut decoder);
            for top in index_top_level(&file) {
                let package = file.name(top.package);
                let stat = td.stat_at(top.entry.addr);
                census_stat(&mut c, &file, &lookup, &mut td, &names, &package, &stat);
            }
        });
        let elapsed = start.elapsed();
        println!("{}: {} TASTy files decoded in {:?}", path, files, elapsed);
        print_census(&c, "  bodies");
        println!();
        total.add(&c);
    }
    if paths.len() > 1 {
        print_census(&total, &format!("all {} jars", paths.len()));
    }
}
