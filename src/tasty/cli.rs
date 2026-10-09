//! `teq tasty`: prints what the reader decodes, which is the debugging tool and what the golden
//! tests under `tests/tasty` compare.

use super::show::Printer;
use super::tags::*;
use super::terms::*;
use super::tree::{index_top_level, Decoder, Entry, TType};
use super::TastyFile;
use crate::zip::Zip;
use std::time::Instant;

fn usage() -> ! {
    eprintln!(
        "usage: teq tasty [--full] <jar or .tasty file> [name-filter]   print the signatures\n\
         \x20      teq tasty --names <.tasty file>                         print the name table\n\
         \x20      teq tasty --trees <jar or .tasty file> [name-filter]    print the raw trees, positions and attributes\n\
         \x20      teq tasty --positions <jar or .tasty file> [name-filter] print every tree's span as scalac's unpickler gives it\n\
         \x20      teq tasty --list <jar>                                  size, CRC-32 and name per entry\n\
         \x20      teq tasty --verify <jar>                                inflate every entry and check it\n\
         \x20      teq tasty --index <jar>...                              open and index, with timings\n\
         \x20      teq tasty --load <jar>...                               load every class and signature through the typer\n\
         \x20      teq tasty --inline-bodies <jar>...                      expand every inline method body through the typer\n\
         \x20      teq tasty --body <jar or .tasty file> <qualified name>   print a definition with its body (@addr: the tree at an address)\n\
         \x20      teq tasty --bodies <jar or .tasty file> [name-filter]    decode every body and report errors\n\
         \x20      teq tasty --term-stats <jar>...                         term tags by occurrence and the census of bodies"
    );
    std::process::exit(2);
}

fn fail(msg: String) -> ! {
    eprintln!("{}", msg);
    std::process::exit(1);
}

/// Calls `f` with the path and bytes of every TASTy file of a jar or of a single file.
pub fn each_tasty(path: &str, filter: Option<&str>, mut f: impl FnMut(&str, Vec<u8>)) {
    if path.ends_with(".tasty") {
        match std::fs::read(path) {
            Ok(bytes) => f(path, bytes),
            Err(e) => fail(format!("cannot read {}: {}", path, e)),
        }
        return;
    }
    let zip = Zip::open(path).unwrap_or_else(|e| fail(e));
    let mut wanted: Vec<usize> = (0..zip.entries.len())
        .filter(|&i| {
            let name = zip.name(&zip.entries[i]);
            name.ends_with(".tasty") && filter.map_or(true, |f| name.contains(f))
        })
        .collect();
    wanted.sort_by(|&a, &b| zip.name(&zip.entries[a]).cmp(zip.name(&zip.entries[b])));
    for i in wanted {
        match zip.read(i) {
            Ok(bytes) => f(zip.name(&zip.entries[i]), bytes),
            Err(e) => fail(e),
        }
    }
}

pub fn print_file(path: &str, bytes: Vec<u8>, full_names: bool, out: &mut String) {
    let size = bytes.len();
    let file = match TastyFile::parse(bytes) {
        Ok(f) => f,
        Err(e) => {
            out.push_str(&format!("// {}: {}\n", path, e));
            return;
        }
    };
    let (major, minor, _) = file.version;
    out.push_str(&format!("// {} (TASTy {}.{}, {} bytes)\n", path, major, minor, size));
    let mut printer = Printer::new(&file, full_names);
    let tops = index_top_level(&file);
    let mut at = 0;
    while at < tops.len() {
        let package = tops[at].package;
        let run = tops[at..].iter().take_while(|t| t.package == package).count();
        printer.out.push_str(&format!("package {}\n", file.name(package)));
        let entries: Vec<_> = tops[at..at + run].iter().map(|t| t.entry.clone()).collect();
        printer.members(&entries, 0);
        at += run;
    }
    out.push_str(&printer.out);
    out.push('\n');
}

/// Prints the definition `qualified` names (`scala.Option.map`, `fix.bodies.Bodies`) with its
/// body from every file that may hold it; returns how many definitions were printed.
fn print_body(path: &str, qualified: &str, full_names: bool, out: &mut String) -> usize {
    let segments: Vec<&str> = qualified.split('.').collect();
    let mut found = 0;
    if path.ends_with(".tasty") {
        let bytes = std::fs::read(path).unwrap_or_else(|e| fail(format!("cannot read {}: {}", path, e)));
        let file = TastyFile::parse(bytes).unwrap_or_else(|e| fail(format!("{}: {}", path, e)));
        // `@223`: the tree at an address, as the signature printer names a body.
        if let Some(addr) = qualified.strip_prefix('@').and_then(|a| a.parse().ok()) {
            let mut decoder = Decoder::new(&file);
            let mut td = TermDecoder::new(&mut decoder);
            let term = td.term_at(addr);
            let mut printer = Printer::new(&file, full_names);
            let text = printer.term(&term, 0);
            out.push_str(&format!("{}\n", text));
            return 1;
        }
        for k in 0..segments.len() {
            found += print_body_in(path, &file, &segments[k..], full_names, out);
            if found > 0 {
                break;
            }
        }
        return found;
    }
    let zip = Zip::open(path).unwrap_or_else(|e| fail(e));
    let names: Vec<String> = zip.entries.iter().map(|e| zip.name(e).to_string()).collect();
    let mut candidates: Vec<(String, usize)> = Vec::new();
    for k in 0..segments.len() {
        let dir = segments[..k].join("/");
        let prefix = if dir.is_empty() { String::new() } else { format!("{}/", dir) };
        candidates.push((format!("{}{}.tasty", prefix, crate::classpath::encode_name(segments[k])), k));
        candidates.push((format!("{}package.tasty", prefix), k));
        for n in &names {
            if n.starts_with(&prefix) && n.ends_with("$package.tasty") && !n[prefix.len()..].contains('/') {
                candidates.push((n.clone(), k));
            }
        }
    }
    for (name, k) in candidates {
        let Some(i) = names.iter().position(|n| *n == name) else { continue };
        let bytes = zip.read(i).unwrap_or_else(|e| fail(e));
        let Ok(file) = TastyFile::parse(bytes) else { continue };
        found += print_body_in(&name, &file, &segments[k..], full_names, out);
    }
    found
}

fn print_body_in(path: &str, file: &TastyFile, rest: &[&str], full_names: bool, out: &mut String) -> usize {
    let tops = index_top_level(file);
    let Some(&first) = rest.first() else { return 0 };
    let mut printer = Printer::new(file, full_names);
    let mut found = 0;
    for top in &tops {
        let e = &top.entry;
        if e.tag == VALDEF && e.flags.has(OBJECT) {
            continue;
        }
        let name = file.name(file.source_name(e.name));
        if name == first {
            found += print_entry_body(&mut printer, e, &rest[1..]);
        } else if e.is_class && (name == "package" || name.ends_with("$package")) {
            found += print_entry_body(&mut printer, e, rest);
        }
    }
    if found > 0 {
        let (major, minor, _) = file.version;
        out.push_str(&format!("// {} (TASTy {}.{})\n", path, major, minor));
        out.push_str(&printer.out);
    }
    found
}

fn print_entry_body(printer: &mut Printer, e: &Entry, rest: &[&str]) -> usize {
    if rest.is_empty() {
        let mut decoder = Decoder::new(printer.file);
        let mut td = TermDecoder::new(&mut decoder);
        let stat = td.stat_at(e.addr);
        let text = printer.stat(&stat, 0);
        printer.out.push_str(&text);
        return 1;
    }
    if !(e.tag == TYPEDEF && e.is_class) {
        return 0;
    }
    let members = printer.decoder.class_sig(e.addr).index.members;
    let mut found = 0;
    for m in &members {
        if m.tag == VALDEF && m.flags.has(OBJECT) {
            continue;
        }
        if printer.file.name(printer.file.source_name(m.name)) == rest[0] {
            found += print_entry_body(printer, m, &rest[1..]);
        }
    }
    found
}

/// The bodies of a file's definitions (see `terms::bodies`), with their decoding errors: an
/// unknown tag, a tree past the depth limit, a shared cycle, an unknown tag inside a type a term
/// carries.
pub struct BodyReport {
    pub bodies: u64,
    pub rhs_bytes: u64,
    pub max_depth: u32,
    pub errors: Vec<String>,
}

pub fn decode_bodies(file: &TastyFile) -> (Vec<Stat>, BodyReport) {
    let mut decoder = Decoder::new(file);
    let mut td = TermDecoder::new(&mut decoder);
    let mut stats = Vec::new();
    let mut packages = Vec::new();
    for top in index_top_level(file) {
        stats.push(td.stat_at(top.entry.addr));
        packages.push(file.name(top.package));
    }
    let mut report = BodyReport { bodies: 0, rhs_bytes: 0, max_depth: td.max_depth, errors: Vec::new() };
    let mut all = Vec::new();
    for (s, package) in stats.iter().zip(&packages) {
        bodies(file, s, package, &mut all);
    }
    report.bodies = all.len() as u64;
    for (_, sig, _) in &all {
        if let Some(b) = sig.and_then(|s| s.body) {
            report.rhs_bytes += td.tree_len(b) as u64;
        }
    }
    let lookup = Decoder::new(file);
    for (owner, sig, body) in &all {
        let mut f = |t: &Term, _: &Scope| {
            match &t.kind {
                TermKind::Unknown(tag, addr) => report.errors.push(format!("{}: unknown tag {} at {}", owner, tag, addr)),
                TermKind::TooDeep(addr) => report.errors.push(format!("{}: tree past the depth limit at {}", owner, addr)),
                TermKind::Cycle(addr) => report.errors.push(format!("{}: shared tree refers to an enclosing tree at {}", owner, addr)),
                TermKind::Path(TType::LocalTerm(addr, _)) if lookup.name_at(*addr).is_none() => {
                    report.errors.push(format!("{}: local reference to no definition at {}", owner, addr))
                }
                _ => {}
            }
            let mut types = Vec::new();
            node_types(t, &mut types);
            if types.iter().any(|ty| type_has_unknown(ty)) {
                report.errors.push(format!("{}: unknown tag in a type of a {}", owner, tag_name(t)));
            }
        };
        Walk { f: &mut f }.term(body, Scope { method: sig.map(|s| s.addr), ..Scope::default() });
    }
    (stats, report)
}

fn decode_all_bodies(path: &str, filter: Option<&str>) {
    let start = Instant::now();
    let (mut files, mut bodies, mut rhs_bytes, mut ast_bytes, mut max_depth) = (0u64, 0u64, 0u64, 0u64, 0u32);
    let mut errors: Vec<String> = Vec::new();
    each_tasty(path, filter, |name, bytes| {
        let file = match TastyFile::parse(bytes) {
            Ok(f) => f,
            Err(e) => {
                errors.push(format!("{}: {}", name, e));
                return;
            }
        };
        files += 1;
        ast_bytes += file.asts.len() as u64;
        let (_, report) = decode_bodies(&file);
        bodies += report.bodies;
        rhs_bytes += report.rhs_bytes;
        max_depth = max_depth.max(report.max_depth);
        errors.extend(report.errors.into_iter().map(|e| format!("{}: {}", name, e)));
    });
    let elapsed = start.elapsed();
    println!(
        "{}: {} TASTy files ({} bytes of trees), {} bodies ({} bytes of right-hand sides) decoded in {:?} ({:.0} bodies per second), deepest term {}, {} errors",
        path,
        files,
        ast_bytes,
        bodies,
        rhs_bytes,
        elapsed,
        bodies as f64 / elapsed.as_secs_f64(),
        max_depth,
        errors.len()
    );
    for e in &errors {
        println!("  {}", e);
    }
    if !errors.is_empty() {
        std::process::exit(1);
    }
}

pub fn run(args: &[String]) -> ! {
    let mut mode = "print";
    let mut full_names = false;
    let mut rest: Vec<&str> = Vec::new();
    for a in args {
        match a.as_str() {
            "--list" => mode = "list",
            "--verify" => mode = "verify",
            "--names" => mode = "names",
            "--trees" => mode = "trees",
            "--positions" => mode = "positions",
            "--index" => mode = "index",
            "--stats" => mode = "stats",
            "--load" => mode = "load",
            "--inline-bodies" => mode = "inline-bodies",
            "--body" => mode = "body",
            "--bodies" => mode = "bodies",
            "--term-stats" => mode = "term-stats",
            "--full" => full_names = true,
            "--signatures" => super::show::SIGNATURES.store(true, std::sync::atomic::Ordering::Relaxed),
            _ if a.starts_with("--") => usage(),
            _ => rest.push(a),
        }
    }
    let Some(&path) = rest.first() else { usage() };
    match mode {
        "list" => {
            let zip = Zip::open(path).unwrap_or_else(|e| fail(e));
            for e in &zip.entries {
                println!("{:>9} {:08x} {}", e.size, e.crc32, zip.name(e));
            }
        }
        "verify" => {
            let start = Instant::now();
            let zip = Zip::open(path).unwrap_or_else(|e| fail(e));
            let mut bytes = 0usize;
            for i in 0..zip.entries.len() {
                bytes += zip.read(i).unwrap_or_else(|e| fail(e)).len();
            }
            println!("{} entries, {} bytes inflated and checked in {:?}", zip.entries.len(), bytes, start.elapsed());
        }
        "names" => each_tasty(path, rest.get(1).copied(), |_, bytes| {
            let file = TastyFile::parse(bytes).unwrap_or_else(|e| fail(e));
            for (i, n) in file.names.iter().enumerate() {
                println!("{:>5} {:<14} {}", i, super::show::name_kind(n), file.name(i as u32));
            }
        }),
        "trees" => each_tasty(path, rest.get(1).copied(), |name, bytes| {
            let file = TastyFile::parse(bytes).unwrap_or_else(|e| fail(format!("{}: {}", name, e)));
            let mut out = format!("// {}\n", name);
            super::dump::dump(&file, &mut out);
            print!("{}", out);
        }),
        "positions" => each_tasty(path, rest.get(1).copied(), |name, bytes| {
            let file = TastyFile::parse(bytes).unwrap_or_else(|e| fail(format!("{}: {}", name, e)));
            print!("{}", super::positions::listing(name, &file));
        }),
        "index" => {
            for &p in &rest {
                let start = Instant::now();
                let zip = Zip::open(p).unwrap_or_else(|e| fail(e));
                let opened = start.elapsed();
                let (mut files, mut tops) = (0, 0);
                each_tasty(p, None, |_, bytes| {
                    if let Ok(file) = TastyFile::parse(bytes) {
                        files += 1;
                        tops += index_top_level(&file).len();
                    }
                });
                println!(
                    "{}: {} entries listed in {:?}; {} TASTy files read and {} top-level definitions indexed in {:?}",
                    p,
                    zip.entries.len(),
                    opened,
                    files,
                    tops,
                    start.elapsed()
                );
            }
        }
        "stats" => super::stats::run(&rest),
        "term-stats" => super::stats::run_terms(&rest),
        "body" => {
            let Some(&name) = rest.get(1) else { usage() };
            let mut out = String::new();
            let found = print_body(path, name, full_names, &mut out);
            print!("{}", out);
            if found == 0 {
                fail(format!("{}: no definition named {}", path, name));
            }
        }
        "bodies" => decode_all_bodies(path, rest.get(1).copied()),
        "load" | "inline-bodies" => {
            let paths: Vec<String> = rest.iter().map(|s| s.to_string()).collect();
            let report = match mode {
                "load" => crate::typer::loader::load_everything(&paths),
                _ => crate::typer::loader::bodies::inline_bodies_report(&paths),
            };
            match report {
                Ok(report) => println!("{}", report),
                Err(e) => fail(e),
            }
        }
        _ => {
            let mut out = String::new();
            each_tasty(path, rest.get(1).copied(), |name, bytes| print_file(name, bytes, full_names, &mut out));
            print!("{}", out);
        }
    }
    std::process::exit(0);
}
