//! The command lines of `teq compiler fix` and `teq fix` (docs/CLI.md).

use super::config::{self, Conf};
use super::engine::{self, Job, Setup};
use std::path::{Path, PathBuf};

/// `teq fix`'s own flags out of `args`.
pub struct Flags {
    pub check: bool,
    pub rules: Option<Vec<String>>,
    pub config: Option<String>,
    /// `--threads n`, compile's option, which both verbs hand to every job they type: its typing's workers, and the
    /// rules' (docs/FIX.md, "Patches, suppressions and rounds").
    pub threads: Option<usize>,
    /// The rest, in order.
    pub rest: Vec<String>,
}

impl Flags {
    /// `--threads n` as a job's compiler options take it, last, so that it wins over a project's own.
    pub fn threads_args(&self) -> Vec<String> {
        self.threads.map(|n| vec!["--threads".to_string(), n.to_string()]).unwrap_or_default()
    }
}

/// `teq fix`'s own flags out of `args`: `--check`, `--rules A,B`, `--config <file>`, `--threads n`.
pub fn flags(args: &[String]) -> Result<Flags, String> {
    let (mut check, mut rules, mut config, mut threads) = (false, None, None, None);
    let mut rest = Vec::new();
    let count = |v: &str| v.parse::<usize>().ok().filter(|&n| n >= 1).ok_or_else(|| format!("--threads takes a number of workers from 1, not {}", v));
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--check" => check = true,
            "--threads" => {
                let Some(v) = args.get(i + 1) else { return Err("--threads needs a value".to_string()) };
                threads = Some(count(v)?);
                i += 1;
            }
            a if a.starts_with("--threads=") => threads = Some(count(&a["--threads=".len()..])?),
            "--rules" | "--config" => {
                let Some(v) = args.get(i + 1) else { return Err(format!("{} needs a value", args[i])) };
                if args[i] == "--rules" {
                    rules = Some(v.split(',').map(|r| r.trim().to_string()).filter(|r| !r.is_empty()).collect());
                } else {
                    config = Some(v.clone());
                }
                i += 1;
            }
            a if a.starts_with("--rules=") => rules = Some(a["--rules=".len()..].split(',').map(|r| r.trim().to_string()).filter(|r| !r.is_empty()).collect()),
            a if a.starts_with("--config=") => config = Some(a["--config=".len()..].to_string()),
            _ => rest.push(args[i].clone()),
        }
        i += 1;
    }
    Ok(Flags { check, rules, config, threads, rest })
}

/// The configuration `--config` names, or `.scalafix.conf` in `dir` where there is one, or none; with the directory
/// its relative `file:` entries start from.
pub fn configuration(named: Option<&str>, dir: &Path) -> Result<(Conf, PathBuf), String> {
    let path = match named {
        Some(p) => PathBuf::from(p),
        None => {
            let p = dir.join(".scalafix.conf");
            if !p.exists() {
                return Ok((Conf::empty(".scalafix.conf"), dir.to_path_buf()));
            }
            p
        }
    };
    let text = std::fs::read_to_string(&path).map_err(|e| format!("cannot read {}: {}", path.display(), e))?;
    let conf = config::parse(&path.to_string_lossy(), &text)?;
    let base = path.parent().map(Path::to_path_buf).filter(|p| !p.as_os_str().is_empty()).unwrap_or_else(|| PathBuf::from("."));
    // Absolute, so that a `file:` entry resolves beside the configuration after `teq fix` works from the build's root.
    let base = std::path::absolute(&base).unwrap_or(base);
    Ok((conf, base))
}

/// `teq compiler fix <inputs> [compiler options] [--check] [--rules A,B] [--config <file>]`: the rules over the
/// program the compiler's own options name, every file of it editable.
pub fn compiler_fix(args: &[String]) -> ! {
    if matches!(args.first().map(String::as_str), Some("--help" | "-h")) {
        println!("{}", USAGE);
        std::process::exit(0);
    }
    let flags = match flags(args) {
        Ok(f) => f,
        Err(e) => {
            eprintln!("teq compiler fix: {}\n{}", e, USAGE);
            std::process::exit(2);
        }
    };
    let mut rest = flags.rest.clone();
    if rest.is_empty() {
        eprintln!("teq compiler fix: no input\n{}", USAGE);
        std::process::exit(2);
    }
    let (conf, config_dir) = match configuration(flags.config.as_deref(), Path::new(".")) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("teq compiler fix: {}", e);
            std::process::exit(2);
        }
    };
    rest.extend(flags.threads_args());
    let code = engine::run(Setup { conf, config_dir, rules: flags.rules, check: flags.check, display: None }, vec![Job { args: rest, roots: None, files: None, generated: Vec::new() }]);
    std::process::exit(code)
}

pub const USAGE: &str = "usage: teq compiler fix <inputs> [compiler options] [--check] [--rules A,B] [--config <file>] [--threads n]";
