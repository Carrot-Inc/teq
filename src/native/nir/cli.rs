//! `teq nir`: prints what the reader decodes from a jar, a directory or a single NIR file, in the
//! text of Scala Native's `nir.Show`: the debugging tool, and what the reader is checked with.
//! `--link` prints in the same text what a program reaches on a class path, which is what the
//! link is checked with, and `--lower` the same lowered to the runtime's object model, which is
//! what the lowering is checked with.

use super::names::Rt;
use super::read::NirFile;
use super::show::Show;
use super::Cx;
use crate::native::classpath::{self, ClassPath};
use crate::native::reach::{self, Config, Triple};
use crate::native::{generate, lower, meta};
use crate::zip::Zip;
use std::io::Write;
use std::path::{Path, PathBuf};

fn usage() -> ! {
    eprintln!(
        "usage: teq nir <jar|dir|.nir> [filter]            print every definition as Scala Native prints it\n\
         \x20      teq nir --verify <jar|dir|.nir> [filter]   decode them, and print their count and the listing's SHA-256\n\
         \x20      teq nir --link <main class> <class path>   print the definitions the program reaches, as Scala Native links them\n\
         \x20      teq nir --lower <main class> <class path>  print them lowered to the runtime's object model, as Scala Native lowers them\n\
         a filter is a substring of the entry's name (java/lang/String.nir)\n\
         a class path is directories and jars joined by '{}', or @<file> for a file that holds them so\n\
         a link takes --triple=<target> ({}) and --threads=<true|false> (true where the program can start a thread)",
        crate::classpath::SEPARATOR,
        crate::native::DEFAULT_TRIPLE
    );
    std::process::exit(2);
}

fn fail(msg: String) -> ! {
    eprintln!("{}", msg);
    std::process::exit(1);
}

/// The NIR files under a directory, each with its name from there. A link to a directory is not
/// followed, so the walk ends with the directories themselves.
fn walk(dir: &Path, prefix: &str, out: &mut Vec<(String, PathBuf)>) -> Result<(), String> {
    let cannot_read = |e: std::io::Error| format!("cannot read {}: {}", dir.display(), e);
    for entry in std::fs::read_dir(dir).map_err(cannot_read)? {
        let entry = entry.map_err(cannot_read)?;
        let name = format!("{}{}", prefix, entry.file_name().to_string_lossy());
        if entry.file_type().map_err(cannot_read)?.is_dir() {
            walk(&entry.path(), &format!("{}/", name), out)?;
        } else if name.ends_with(".nir") {
            out.push((name, entry.path()));
        }
    }
    Ok(())
}

/// One NIR file of an input: its entry name, which the filter and the listing use, what a message
/// calls it, and its bytes, read when called for.
struct Item {
    name: String,
    label: String,
    read: Box<dyn Fn() -> Result<Vec<u8>, String>>,
}

fn file_item(name: String, file: PathBuf) -> Item {
    let label = file.display().to_string();
    Item { name, label, read: Box::new(move || std::fs::read(&file).map_err(|e| format!("cannot read {}: {}", file.display(), e))) }
}

/// The NIR files of an input, in their names' order as Java orders strings.
fn items(input: &str, filter: Option<&str>) -> Result<Vec<Item>, String> {
    let path = Path::new(input);
    let mut found = Vec::new();
    if path.is_dir() {
        let mut entries = Vec::new();
        walk(path, "", &mut entries)?;
        found.extend(entries.into_iter().map(|(name, file)| file_item(name, file)));
    } else if input.ends_with(".nir") {
        found.push(file_item(input.to_string(), path.to_path_buf()));
    } else {
        let zip = std::rc::Rc::new(Zip::open(input)?);
        for (index, e) in zip.entries.iter().enumerate() {
            let name = zip.name(e);
            if name.ends_with(".nir") {
                let zip = zip.clone();
                found.push(Item { name: name.to_string(), label: format!("{}: {}", input, name), read: Box::new(move || zip.read(index)) });
            }
        }
    }
    found.retain(|item| filter.is_none_or(|f| item.name.contains(f)));
    found.sort_by(|a, b| super::java_order(a.name.as_bytes(), b.name.as_bytes()));
    Ok(found)
}

/// The program's definitions after the link, sorted by name, each as `nir.Show` prints it: the
/// text Scala Native's build dumps as `linked.hnir`; lowered, the one it dumps as `lowered.hnir`.
fn link(args: &[String], lowers: bool) -> ! {
    let (mut triple, mut threads, mut plain) = (crate::native::DEFAULT_TRIPLE.to_string(), None, Vec::new());
    for arg in args {
        match arg.split_once('=') {
            Some(("--triple", target)) => triple = target.to_string(),
            Some(("--threads", "true")) => threads = Some(true),
            Some(("--threads", "false")) => threads = Some(false),
            _ if arg.starts_with("--") => usage(),
            _ => plain.push(arg.as_str()),
        }
    }
    let [main, class_path] = plain[..] else { usage() };
    let paths = classpath::entries(class_path);
    let mut cx = Cx::new();
    let rt = Rt::new(&mut cx);
    let cp = ClassPath::open(&mut cx, &paths).unwrap_or_else(|e| fail(e));
    // A debug build with the default collector, as Scala Native's build links one.
    let config = Config { main: main.to_string(), multithreading: threads, release: false, gc: "immix", triple: Triple::parse(&triple), link_stubs: false, service_providers: Vec::new() };
    let linked = reach::link(&mut cx, &rt, &cp, &config).unwrap_or_else(|e| fail(e));
    let lowered = lowers.then(|| {
        let main = cx.top(main);
        let proxies = generate::reflective_proxies(&mut cx, &rt, &linked);
        let meta = meta::Meta::new(&mut cx, &rt, &linked, &proxies).unwrap_or_else(|e| fail(e));
        let generated = generate::generate(&mut cx, &rt, &linked, &meta, &proxies, main).unwrap_or_else(|e| fail(e));
        let mut out = Vec::new();
        let mut lowering = lower::Lowering::new(&mut cx, &rt, &linked, &meta).unwrap_or_else(|e| fail(e));
        for (defns, this_is_used) in [(&linked.defns, false), (&proxies, true), (&generated, false)] {
            lowering.defns(defns, this_is_used, &mut out).unwrap_or_else(|e| fail(e));
        }
        drop(lowering);
        out.sort_by(|a, b| cx.cmp_globals(a.name, b.name));
        out
    });
    let stdout = std::io::stdout();
    let mut out = std::io::BufWriter::with_capacity(1 << 16, stdout.lock());
    for defn in lowered.as_ref().unwrap_or(&linked.defns) {
        let mut show = Show::new(&cx);
        show.defn(defn);
        show.out.push(b'\n');
        if out.write_all(&show.out).is_err() {
            // A closed pipe ends the listing quietly.
            std::process::exit(0);
        }
    }
    let _ = out.flush();
    std::process::exit(0);
}

pub fn run(args: &[String]) -> ! {
    match args.first().map(String::as_str) {
        Some("--link") => link(&args[1..], false),
        Some("--lower") => link(&args[1..], true),
        _ => {}
    }
    let (verify, rest) = match args.first().map(String::as_str) {
        Some("--verify") => (true, &args[1..]),
        _ => (false, args),
    };
    let (input, filter) = match rest {
        [input] if !input.starts_with("--") => (input, None),
        [input, filter] if !input.starts_with("--") => (input, Some(filter.as_str())),
        _ => usage(),
    };
    let items = items(input, filter).unwrap_or_else(|e| fail(e));
    let mut cx = Cx::new();
    let stdout = std::io::stdout();
    let mut out = std::io::BufWriter::with_capacity(1 << 16, stdout.lock());
    let mut count = 0usize;
    let mut digest = crate::task::sha256::Sha256::new();
    for item in &items {
        let bytes = (item.read)().unwrap_or_else(|e| fail(e));
        let defns = NirFile::open(&bytes).and_then(|mut f| f.defns(&mut cx)).unwrap_or_else(|e| fail(format!("{}: {}", item.label, e)));
        count += defns.len();
        let mut shown: Vec<(Vec<u8>, &super::Defn)> = defns
            .iter()
            .map(|d| {
                let mut mangled = Vec::new();
                cx.mangle(d.name, &mut mangled);
                (mangled, d)
            })
            .collect();
        shown.sort_by(|a, b| super::java_order(&a.0, &b.0));
        let mut show = Show::new(&cx);
        show.out.extend_from_slice(format!("## {}\n", item.name).as_bytes());
        for (_, defn) in shown {
            show.defn(defn);
            show.out.push(b'\n');
        }
        if verify {
            digest.update(&show.out);
        } else if out.write_all(&show.out).is_err() {
            // A closed pipe ends the listing quietly.
            std::process::exit(0);
        }
    }
    if verify {
        let _ = writeln!(out, "{} files, {} definitions, listing sha256 {}", items.len(), count, digest.hex());
    }
    let _ = out.flush();
    std::process::exit(0);
}
