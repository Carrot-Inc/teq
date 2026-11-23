//! `teq nir`: prints what the reader decodes from a jar, a directory or a single NIR file, in the
//! text of Scala Native's `nir.Show`: the debugging tool, and what the reader is checked with.
//! `--link` prints in the same text what a program reaches on a class path, which is what the
//! link is checked with, and `--lower` the same lowered to the runtime's object model, which is
//! what the lowering is checked with; `--ll` writes the lowered program as LLVM IR instead, and
//! `--build` the executable built from it.

use super::names::Rt;
use super::read::NirFile;
use super::show::Show;
use super::{Cx, Str};
use crate::native::classpath::{self, ClassPath};
use crate::native::reach::{self, Config, Triple};
use crate::native::{build, codegen, generate, lower, meta, tools};
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
         a link takes --triple=<target> ({}) and --threads=<true|false> (true where the program can start a thread)\n\
         --lower takes --ll=<directory>: the program written there as LLVM IR in the listing's place, a module for each directory of sources\n\
         --link takes --build=<executable> and --work=<directory>: the program built there by zig, a static x86_64-linux-musl executable,\n\
         \x20 with --zig=<path> (else TEQ_ZIG) naming zig {}, --zig-cache=<directory> (<work>/zig-cache) and --lib-dir=<directory>,\n\
         \x20 each a directory of lib<name>.a archives with their headers under include/",
        crate::classpath::SEPARATOR,
        crate::native::DEFAULT_TRIPLE,
        tools::ZIG_VERSION
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
/// text Scala Native's build dumps as `linked.hnir`; lowered, the one it dumps as `lowered.hnir`,
/// or its LLVM IR, the modules Scala Native's build writes; or the executable built from them.
fn link(args: &[String], lowers: bool) -> ! {
    let (mut triple, mut threads, mut ll_dir, mut plain) = (crate::native::DEFAULT_TRIPLE.to_string(), None, None, Vec::new());
    let (mut output, mut work, mut zig, mut zig_cache, mut lib_dirs) = (None, None, None, None, Vec::new());
    for arg in args {
        match arg.split_once('=') {
            Some(("--triple", target)) => triple = target.to_string(),
            Some(("--ll", dir)) if lowers => ll_dir = Some(dir.to_string()),
            Some(("--build", file)) if !lowers => output = Some(PathBuf::from(file)),
            Some(("--work", dir)) if !lowers => work = Some(PathBuf::from(dir)),
            Some(("--zig", file)) if !lowers => zig = Some(PathBuf::from(file)),
            Some(("--zig-cache", dir)) if !lowers => zig_cache = Some(PathBuf::from(dir)),
            Some(("--lib-dir", dir)) if !lowers => lib_dirs.push(PathBuf::from(dir)),
            Some(("--threads", "true")) => threads = Some(true),
            Some(("--threads", "false")) => threads = Some(false),
            _ if arg.starts_with("--") => usage(),
            _ => plain.push(arg.as_str()),
        }
    }
    let [main, class_path] = plain[..] else { usage() };
    if output.is_none() && (work.is_some() || zig.is_some() || zig_cache.is_some() || !lib_dirs.is_empty()) {
        usage();
    }
    // What a build needs is checked before the link, which takes a while.
    let building = output.map(|output| {
        let absolute = |path: &Path| std::path::absolute(path).unwrap_or_else(|_| path.to_path_buf());
        let work = absolute(&work.unwrap_or_else(|| fail("--build needs --work=<directory>".to_string())));
        build::Options {
            zig: tools::zig(zig.as_deref()).unwrap_or_else(|e| fail(e)),
            zig_cache: zig_cache.map_or_else(|| work.join("zig-cache"), |dir| absolute(&dir)),
            target: build::target(&Triple::parse(&triple)).unwrap_or_else(|e| fail(e)),
            output: absolute(&output),
            lib_dirs: lib_dirs.iter().map(|dir| absolute(dir)).collect(),
            lib_dir_option: "--lib-dir=<dir>".to_string(),
            workers: std::thread::available_parallelism().map_or(1, |n| n.get()),
            work,
        }
    });
    let paths = classpath::entries(class_path);
    let mut cx = Cx::new();
    let rt = Rt::new(&mut cx);
    let cp = ClassPath::open(&mut cx, &paths).unwrap_or_else(|e| fail(e));
    // A debug build with the default collector, as Scala Native's build links one.
    let config = Config { main: main.to_string(), multithreading: threads, release: false, gc: "immix", triple: Triple::parse(&triple), link_stubs: false, service_providers: Vec::new() };
    let linked = reach::link(&mut cx, &rt, &cp, &config).unwrap_or_else(|e| fail(e));
    let lowered = (lowers || building.is_some()).then(|| {
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
        if let Some(dir) = &ll_dir {
            let modules = codegen::modules(&cx, &rt, &linked, &meta, &out, &triple).unwrap_or_else(|e| fail(e));
            std::fs::create_dir_all(dir).unwrap_or_else(|e| fail(format!("cannot write {}: {}", dir, e)));
            for module in modules {
                let path = Path::new(dir).join(format!("{}.ll", module.name));
                std::fs::write(&path, module.text).unwrap_or_else(|e| fail(format!("cannot write {}: {}", path.display(), e)));
            }
            std::process::exit(0);
        }
        if let Some(options) = &building {
            let modules = codegen::modules(&cx, &rt, &linked, &meta, &out, &triple).unwrap_or_else(|e| fail(e));
            let text = |names: &[Str]| names.iter().map(|s| String::from_utf8_lossy(cx.bytes(*s)).into_owned()).collect::<Vec<_>>();
            let program = build::Program { modules: &modules, links: text(&linked.links), defines: text(&linked.defines), multithreading: linked.multithreading, link_cpp_runtime: linked.link_cpp_runtime, gc: config.gc };
            build::build(&paths, &program, options).unwrap_or_else(|e| fail(e));
            std::process::exit(0);
        }
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
