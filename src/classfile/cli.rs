//! `teq classfile`: prints what the reader decodes from a jar, a directory, a single class file
//! or the JDK's `ct.sym`, which is the debugging tool and what the golden tests under
//! `tests/classfile` compare.

use super::jdk::{self, CtSym};
use super::show::print_class;
use super::ClassFile;
use crate::zip::Zip;
use std::path::{Path, PathBuf};
use std::time::Instant;

fn usage() -> ! {
    eprintln!(
        "usage: teq classfile [--full] <jar|dir|ct.sym|.class> [filter]   print the signatures\n\
         \x20      teq classfile --jdk [filter]                            the same for the JDK's lib/ct.sym\n\
         \x20      teq classfile --list <input>                            size, CRC-32 and name per class file\n\
         \x20      teq classfile --verify <input> [filter]                 parse every class file\n\
         \x20      teq classfile --stats <input> [filter]                  which signatures map onto teq's types\n\
         \x20      teq classfile --bench <input>                           open, list and parse, with timings\n\
         \x20      teq classfile --jdk-path                                where the JDK's ct.sym was found\n\
         \x20      --release <n>   the release to read from ct.sym (default: the newest it holds)\n\
         a filter is a substring of the class name (java.base/java/lang/String for ct.sym), or an\n\
         entry name ending in .sig or .class for one class"
    );
    std::process::exit(2);
}

fn fail(msg: String) -> ! {
    eprintln!("{}", msg);
    std::process::exit(1);
}

pub enum Input {
    CtSym { ct: CtSym, release: u32 },
    Jar(Zip),
    Dir(PathBuf),
    File(PathBuf),
}

/// One class file of an input: its entry name and the name the filter and the listing use.
pub struct Item {
    pub entry: String,
    pub name: String,
    pub index: usize,
    pub size: u32,
    pub crc32: u32,
}

const MAX_DIR_DEPTH: u32 = 32;

impl Input {
    pub fn open(path: &str, release: Option<u32>) -> Result<Input, String> {
        let p = Path::new(path);
        if p.is_dir() {
            return Ok(Input::Dir(p.to_path_buf()));
        }
        if path.ends_with(".class") || path.ends_with(".sig") {
            return Ok(Input::File(p.to_path_buf()));
        }
        if path.ends_with("ct.sym") {
            let ct = CtSym::open(path)?;
            let release = match release {
                Some(r) if ct.releases.contains(&r) => r,
                Some(r) => {
                    return Err(format!("{}: release {} is not in it (it holds {} to {})", path, r, ct.releases[0], ct.latest_release()))
                }
                None => ct.latest_release(),
            };
            return Ok(Input::CtSym { ct, release });
        }
        Ok(Input::Jar(Zip::open(path)?))
    }

    pub fn jdk(release: Option<u32>) -> Result<Input, String> {
        let home = jdk::find_home().ok_or("no JDK found: set JAVA_HOME or put java on the path")?;
        let path = jdk::ct_sym_path(&home);
        Input::open(&path.to_string_lossy(), release)
    }

    pub fn describe(&self) -> String {
        match self {
            Input::CtSym { ct, release } => format!("ct.sym release {} (of {} to {})", release, ct.releases[0], ct.latest_release()),
            Input::Jar(_) => "jar".to_string(),
            Input::Dir(_) => "directory".to_string(),
            Input::File(_) => "class file".to_string(),
        }
    }

    /// The class files, sorted by name, that the filter keeps.
    pub fn items(&self, filter: Option<&str>) -> Result<Vec<Item>, String> {
        let keep = |entry: &str, name: &str| match filter {
            None => true,
            Some(f) if f.ends_with(".sig") || f.ends_with(".class") => entry == f || entry.ends_with(&format!("/{}", f)),
            Some(f) => name.contains(f),
        };
        let mut items = Vec::new();
        match self {
            Input::CtSym { ct, release } => {
                for s in ct.classes(*release, "") {
                    let e = &ct.zip.entries[s.entry];
                    let entry = ct.zip.name(e).to_string();
                    let name = format!("{}/{}", s.module, s.class);
                    if keep(&entry, &name) {
                        items.push(Item { entry, name, index: s.entry, size: e.size, crc32: e.crc32 });
                    }
                }
            }
            Input::Jar(zip) => {
                for (i, e) in zip.entries.iter().enumerate() {
                    let entry = zip.name(e);
                    let Some(name) = entry.strip_suffix(".class") else { continue };
                    if keep(entry, name) {
                        items.push(Item { entry: entry.to_string(), name: name.to_string(), index: i, size: e.size, crc32: e.crc32 });
                    }
                }
                items.sort_by(|a, b| a.name.cmp(&b.name));
            }
            Input::Dir(dir) => {
                let mut paths = Vec::new();
                walk_dir(dir, 0, &mut paths)?;
                paths.sort();
                for p in paths {
                    let rel = p.strip_prefix(dir).unwrap_or(&p).to_string_lossy().replace('\\', "/");
                    let name = rel.strip_suffix(".class").or_else(|| rel.strip_suffix(".sig")).unwrap_or(&rel).to_string();
                    if keep(&rel, &name) {
                        let size = std::fs::metadata(&p).map(|m| m.len() as u32).unwrap_or(0);
                        items.push(Item { entry: p.to_string_lossy().into_owned(), name, index: 0, size, crc32: 0 });
                    }
                }
            }
            Input::File(p) => {
                let entry = p.to_string_lossy().into_owned();
                let name = p.file_stem().map_or(String::new(), |s| s.to_string_lossy().into_owned());
                let size = std::fs::metadata(p).map(|m| m.len() as u32).unwrap_or(0);
                items.push(Item { entry, name, index: 0, size, crc32: 0 });
            }
        }
        Ok(items)
    }

    /// The class files written by javac, by the central directory alone: a class file with a
    /// `.tasty` sibling for its stem or for a `$`-prefix of its stem (`Foo$.class`,
    /// `Foo$Inner.class`, `$colon$colon$.class`) was written by a Scala compiler, whose TASTy
    /// is what a loader reads instead. Every other input is taken whole.
    pub fn java_items(&self, filter: Option<&str>) -> Result<Vec<Item>, String> {
        let mut items = self.items(filter)?;
        let Input::Jar(zip) = self else { return Ok(items) };
        let tasty: std::collections::HashSet<&str> =
            zip.entries.iter().filter_map(|e| zip.name(e).strip_suffix(".tasty")).collect();
        items.retain(|item| {
            let mut stem = item.name.as_str();
            loop {
                if tasty.contains(stem) {
                    return false;
                }
                match stem.rfind('$') {
                    Some(at) if at > 0 => stem = &stem[..at],
                    _ => return true,
                }
            }
        });
        Ok(items)
    }

    pub fn read(&self, item: &Item) -> Result<Vec<u8>, String> {
        match self {
            Input::CtSym { ct, .. } => ct.zip.read(item.index),
            Input::Jar(zip) => zip.read(item.index),
            Input::Dir(_) | Input::File(_) => std::fs::read(&item.entry).map_err(|e| format!("cannot read {}: {}", item.entry, e)),
        }
    }
}

fn walk_dir(dir: &Path, depth: u32, out: &mut Vec<PathBuf>) -> Result<(), String> {
    if depth > MAX_DIR_DEPTH {
        return Err(format!("{}: directories nested too deeply", dir.display()));
    }
    let entries = std::fs::read_dir(dir).map_err(|e| format!("cannot read {}: {}", dir.display(), e))?;
    for entry in entries {
        let p = entry.map_err(|e| e.to_string())?.path();
        if p.is_dir() {
            walk_dir(&p, depth + 1, out)?;
        } else if p.extension().map_or(false, |x| x == "class" || x == "sig") {
            out.push(p);
        }
    }
    Ok(())
}

pub struct Args {
    pub mode: String,
    pub full_names: bool,
    pub release: Option<u32>,
    pub jdk: bool,
    pub rest: Vec<String>,
}

fn parse_args(args: &[String]) -> Args {
    let mut a = Args { mode: "print".to_string(), full_names: false, release: None, jdk: false, rest: Vec::new() };
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--list" | "--verify" | "--stats" | "--bench" | "--jdk-path" => a.mode = args[i][2..].to_string(),
            "--full" => a.full_names = true,
            "--jdk" => a.jdk = true,
            "--release" => {
                i += 1;
                a.release = args.get(i).and_then(|r| r.parse().ok()).or_else(|| usage());
            }
            s if s.starts_with("--") => usage(),
            s => a.rest.push(s.to_string()),
        }
        i += 1;
    }
    a
}

fn open(a: &Args) -> (Input, Option<&str>) {
    if a.jdk {
        return (Input::jdk(a.release).unwrap_or_else(|e| fail(e)), a.rest.first().map(String::as_str));
    }
    let Some(path) = a.rest.first() else { usage() };
    (Input::open(path, a.release).unwrap_or_else(|e| fail(e)), a.rest.get(1).map(String::as_str))
}

pub fn run(args: &[String]) -> ! {
    let a = parse_args(args);
    if a.mode == "jdk-path" {
        match jdk::find_home() {
            Some(home) => println!("{}", jdk::ct_sym_path(&home).display()),
            None => fail("no JDK found: set JAVA_HOME or put java on the path".to_string()),
        }
        std::process::exit(0);
    }
    let (input, filter) = open(&a);
    match a.mode.as_str() {
        "list" => {
            for item in input.items(filter).unwrap_or_else(|e| fail(e)) {
                println!("{:>9} {:08x} {}", item.size, item.crc32, item.entry);
            }
        }
        "verify" => {
            let start = Instant::now();
            let items = input.items(filter).unwrap_or_else(|e| fail(e));
            let (mut bytes, mut failed, mut scala) = (0usize, 0usize, 0usize);
            for item in &items {
                let data = input.read(item).unwrap_or_else(|e| fail(e));
                bytes += data.len();
                match ClassFile::parse(&data) {
                    Ok(cf) => {
                        if cf.scala {
                            scala += 1;
                        }
                        if let Err(e) = super::stats::check_signatures(&cf) {
                            failed += 1;
                            println!("{}: {}", item.entry, e);
                        }
                    }
                    Err(e) => {
                        failed += 1;
                        println!("{}: {}", item.entry, e);
                    }
                }
            }
            println!(
                "{}: {} class files ({} bytes) parsed in {:?}, {} written by a Scala compiler, {} failed",
                input.describe(),
                items.len(),
                bytes,
                start.elapsed(),
                scala,
                failed
            );
            if failed > 0 {
                std::process::exit(1);
            }
        }
        "stats" => super::stats::run(&input, filter),
        "bench" => bench(&a),
        _ => {
            let mut out = String::new();
            let mut failed = false;
            for item in input.items(filter).unwrap_or_else(|e| fail(e)) {
                let data = input.read(&item).unwrap_or_else(|e| fail(e));
                if let Err(e) = print_class(&item.entry, &data, a.full_names, &mut out) {
                    out.push_str(&format!("// {}: {}\n", item.entry, e));
                    failed = true;
                }
                out.push('\n');
            }
            print!("{}", out);
            if failed {
                std::process::exit(1);
            }
        }
    }
    std::process::exit(0);
}

/// The measurements: what a loader pays to open the input, list a package
/// and read a class, and the worst case of reading every class.
fn bench(a: &Args) {
    let start = Instant::now();
    let (input, _) = open(a);
    let opened = start.elapsed();
    match &input {
        Input::CtSym { ct, release } => {
            println!("ct.sym opened in {:?}: {} entries, releases {} to {}", opened, ct.zip.entries.len(), ct.releases[0], ct.latest_release());
            let start = Instant::now();
            let lang = ct.package(*release, "java/lang");
            println!("  java.lang of release {}: {} classes listed in {:?}", release, lang.len(), start.elapsed());
            let start = Instant::now();
            let index = ct.index(*release);
            let built = start.elapsed();
            let start = Instant::now();
            let n = index.get("java/lang").map_or(0, Vec::len);
            println!("  index of release {}: {} packages built in {:?}; java.lang looked up ({} classes) in {:?}", release, index.len(), built, n, start.elapsed());
            parse_named(&input, &["java.base/java/lang/String.sig", "java.base/java/util/Map.sig", "java.base/java/lang/Object.sig"]);
            parse_all(&input, Some("java.base/"));
        }
        _ => {
            println!("{} opened in {:?}", input.describe(), opened);
            parse_named(&input, &["scala/math/ScalaNumber.class", "scala/runtime/BoxesRunTime.class"]);
            parse_all(&input, None);
        }
    }
}

fn parse_named(input: &Input, names: &[&str]) {
    for n in names {
        let start = Instant::now();
        let items = input.items(Some(n)).unwrap_or_else(|e| fail(e));
        let Some(item) = items.first() else {
            println!("  {}: not found", n);
            continue;
        };
        let found = start.elapsed();
        let start = Instant::now();
        let data = input.read(item).unwrap_or_else(|e| fail(e));
        let read = start.elapsed();
        let start = Instant::now();
        let cf = ClassFile::parse(&data).unwrap_or_else(|e| fail(e));
        let sigs = super::stats::check_signatures(&cf).unwrap_or_else(|e| fail(e));
        println!(
            "  {}: found in {:?}, {} bytes read and inflated in {:?}, parsed and {} signatures decoded in {:?}",
            item.entry,
            found,
            data.len(),
            read,
            sigs,
            start.elapsed()
        );
    }
}

fn parse_all(input: &Input, filter: Option<&str>) {
    let start = Instant::now();
    let items = input.java_items(filter).unwrap_or_else(|e| fail(e));
    let listed = start.elapsed();
    let (mut bytes, mut scala, mut sigs) = (0usize, 0usize, 0usize);
    for item in &items {
        let data = input.read(item).unwrap_or_else(|e| fail(e));
        bytes += data.len();
        let cf = ClassFile::parse(&data).unwrap_or_else(|e| fail(format!("{}: {}", item.entry, e)));
        scala += cf.scala as usize;
        sigs += super::stats::check_signatures(&cf).unwrap_or_else(|e| fail(format!("{}: {}", item.entry, e)));
    }
    println!(
        "  every Java class{}: {} selected by name in {:?} ({} of them carry a Scala attribute); {} bytes read, parsed and {} signatures decoded in {:?}",
        filter.map_or(String::new(), |f| format!(" of {}", f)),
        items.len(),
        listed,
        scala,
        bytes,
        sigs,
        start.elapsed()
    );
}
