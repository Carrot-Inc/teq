//! `teq nir`: prints what the reader decodes from a jar, a directory or a single NIR file, in the
//! text of Scala Native's `nir.Show`: the debugging tool, and what the reader is checked with.

use super::read::NirFile;
use super::show::Show;
use super::Cx;
use crate::zip::Zip;
use std::io::Write;
use std::path::{Path, PathBuf};

fn usage() -> ! {
    eprintln!(
        "usage: teq nir <jar|dir|.nir> [filter]            print every definition as Scala Native prints it\n\
         \x20      teq nir --verify <jar|dir|.nir> [filter]   decode them, and print their count and the listing's SHA-256\n\
         a filter is a substring of the entry's name (java/lang/String.nir)"
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

pub fn run(args: &[String]) -> ! {
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
