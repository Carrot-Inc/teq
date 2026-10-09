//! Dialect flags: each turns a costly construct off for a code base and names the cost it
//! removes. They come from the command line (`--dialect no-overloading,no-inline`, `--dialect
//! strict`) and from a `teq.toml` next to the sources:
//!
//! ```toml
//! dialect = "strict"        # the base: every flag on; "full", the default, has none
//!
//! [dialect]
//! no-overloading = true     # single flags on top of the base
//! strict-equality = false
//! ```
//!
//! `strict` is the dialect teq spoke before overloading, conversions, Scala 2 implicits, inline
//! and inferred member types were supported.

use std::path::Path;

#[derive(Clone, Copy, Default, PartialEq, Eq, Debug)]
pub struct Dialect {
    pub no_implicit_conversions: bool,
    pub no_overloading: bool,
    pub no_inline: bool,
    pub explicit_result_types: bool,
    pub no_scala2_implicits: bool,
    pub strict_equality: bool,
    pub no_nonlocal_returns: bool,
    /// An extension rather than a restriction: an `inline val`, an `inline if` condition and an
    /// `inline match` scrutinee may be any pure expression over the standard library, which the
    /// interpreter evaluates; scalac accepts constants only.
    pub interpreted_constants: bool,
}

pub const NO_IMPLICIT_CONVERSIONS: &str = "no-implicit-conversions";
pub const NO_OVERLOADING: &str = "no-overloading";
pub const NO_INLINE: &str = "no-inline";
pub const EXPLICIT_RESULT_TYPES: &str = "explicit-result-types";
pub const NO_SCALA2_IMPLICITS: &str = "no-scala2-implicits";
pub const STRICT_EQUALITY: &str = "strict-equality";
pub const NO_NONLOCAL_RETURNS: &str = "no-nonlocal-returns";
pub const INTERPRETED_CONSTANTS: &str = "interpreted-constants";

pub const FLAGS: [&str; 8] =
    [NO_IMPLICIT_CONVERSIONS, NO_OVERLOADING, NO_INLINE, EXPLICIT_RESULT_TYPES, NO_SCALA2_IMPLICITS, STRICT_EQUALITY, NO_NONLOCAL_RETURNS, INTERPRETED_CONSTANTS];

impl Dialect {
    pub fn strict() -> Dialect {
        Dialect {
            no_implicit_conversions: true,
            no_overloading: true,
            no_inline: true,
            explicit_result_types: true,
            no_scala2_implicits: true,
            strict_equality: true,
            no_nonlocal_returns: true,
            interpreted_constants: false,
        }
    }

    pub fn any(&self) -> bool {
        *self != Dialect::default()
    }

    fn flag(&mut self, name: &str) -> Option<&mut bool> {
        Some(match name {
            NO_IMPLICIT_CONVERSIONS => &mut self.no_implicit_conversions,
            NO_OVERLOADING => &mut self.no_overloading,
            NO_INLINE => &mut self.no_inline,
            EXPLICIT_RESULT_TYPES => &mut self.explicit_result_types,
            NO_SCALA2_IMPLICITS => &mut self.no_scala2_implicits,
            STRICT_EQUALITY => &mut self.strict_equality,
            NO_NONLOCAL_RETURNS => &mut self.no_nonlocal_returns,
            INTERPRETED_CONSTANTS => &mut self.interpreted_constants,
            _ => return None,
        })
    }

    pub fn set(&mut self, name: &str, on: bool) -> Result<(), String> {
        match self.flag(name) {
            Some(flag) => {
                *flag = on;
                Ok(())
            }
            None => Err(format!("unknown dialect flag `{}`; the flags are {} and the dialect `strict`", name, FLAGS.join(", "))),
        }
    }

    /// `--dialect strict` or a comma-separated list of flags, added to what is set already.
    pub fn add_flags(&mut self, list: &str) -> Result<(), String> {
        for name in list.split(',').map(str::trim).filter(|n| !n.is_empty()) {
            match name {
                "strict" => *self = Dialect::strict(),
                "full" => *self = Dialect::default(),
                _ => self.set(name, true)?,
            }
        }
        Ok(())
    }

    /// The dialect of a `teq.toml`: the base named by a top-level `dialect` and the flags of its
    /// `[dialect]` table. Other tables and keys are not read.
    pub fn from_toml(text: &str) -> Result<Dialect, String> {
        let mut dialect = Dialect::default();
        let mut section = String::new();
        for (i, raw) in text.lines().enumerate() {
            let line = strip_comment(raw).trim();
            if line.is_empty() {
                continue;
            }
            if let Some(header) = line.strip_prefix('[') {
                section = header.trim_end_matches(']').trim().to_string();
                continue;
            }
            let Some((key, value)) = line.split_once('=') else {
                return Err(format!("line {}: expected `key = value`", i + 1));
            };
            let (key, value) = (key.trim().trim_matches('"'), value.trim());
            match (section.as_str(), key) {
                ("", "dialect") => match unquote(value) {
                    Some("strict") => dialect = Dialect::strict(),
                    Some("full") => dialect = Dialect::default(),
                    Some(other) => return Err(format!("line {}: unknown dialect \"{}\"; the dialects are \"strict\" and \"full\"", i + 1, other)),
                    None => return Err(format!("line {}: `dialect` takes a quoted name", i + 1)),
                },
                ("dialect", flag) => {
                    let on = match value {
                        "true" => true,
                        "false" => false,
                        _ => return Err(format!("line {}: `{}` takes true or false", i + 1, flag)),
                    };
                    dialect.set(flag, on).map_err(|e| format!("line {}: {}", i + 1, e))?;
                }
                _ => {}
            }
        }
        Ok(dialect)
    }

    /// The `teq.toml` next to the sources: in the first input directory that has one, or next to
    /// the first input file that has one.
    pub fn find_toml(inputs: &[String]) -> Option<(String, String)> {
        for input in inputs {
            let p = Path::new(input);
            let dir = if p.is_dir() { p } else { p.parent().unwrap_or(Path::new(".")) };
            let toml = dir.join("teq.toml");
            if let Ok(text) = std::fs::read_to_string(&toml) {
                return Some((toml.to_string_lossy().into_owned(), text));
            }
        }
        None
    }
}

/// The top-level `max-inlines = n` of a `teq.toml`, scalac's `-Xmax-inlines`.
pub fn max_inlines_from_toml(text: &str) -> Result<Option<u32>, String> {
    for (i, raw) in text.lines().enumerate() {
        let line = strip_comment(raw).trim();
        if line.starts_with('[') {
            break;
        }
        let Some((key, value)) = line.split_once('=') else { continue };
        if key.trim().trim_matches('"') != "max-inlines" {
            continue;
        }
        return match value.trim().parse::<u32>() {
            Ok(n) if n > 0 => Ok(Some(n)),
            _ => Err(format!("line {}: `max-inlines` takes a positive number", i + 1)),
        };
    }
    Ok(None)
}

/// The top-level `cacheable-state = ["a.B", ...]` of a `teq.toml`: the objects declared to hold
/// cacheable state (`--cacheable-state`).
pub fn cacheable_state_from_toml(text: &str) -> Result<Vec<String>, String> {
    for (i, raw) in text.lines().enumerate() {
        let line = strip_comment(raw).trim();
        if line.starts_with('[') {
            break;
        }
        let Some((key, value)) = line.split_once('=') else { continue };
        if key.trim().trim_matches('"') != "cacheable-state" {
            continue;
        }
        let value = value.trim();
        let Some(list) = value.strip_prefix('[').and_then(|v| v.strip_suffix(']')) else {
            return Err(format!("line {}: `cacheable-state` takes a list of quoted names", i + 1));
        };
        let mut names = Vec::new();
        for item in list.split(',').map(str::trim).filter(|s| !s.is_empty()) {
            match unquote(item) {
                Some(name) if !name.is_empty() => names.push(name.to_string()),
                _ => return Err(format!("line {}: `cacheable-state` takes a list of quoted names", i + 1)),
            }
        }
        return Ok(names);
    }
    Ok(Vec::new())
}

/// The cost a flag removes, for the messages of the constructs it rejects and the hints of
/// `--profile`.
pub fn cost(flag: &str) -> &'static str {
    match flag {
        NO_IMPLICIT_CONVERSIONS => {
            "a conversion is searched for at every selection of a member the receiver lacks and at every argument or result whose type does not conform"
        }
        NO_OVERLOADING => "every call of an overloaded name is resolved among its alternatives by shape and by applicability",
        NO_INLINE => "a call of an inline method types the method's body again at the call site, and the inline calls in that body with it",
        EXPLICIT_RESULT_TYPES => {
            "an inferred type makes the type of a definition depend on its body, which is typed before any use of it, and an edit of the body can change every use"
        }
        NO_SCALA2_IMPLICITS => {
            "a Scala 2 implicit joins the candidates of every given search that a wildcard import of its scope reaches, and an implicit def with a plain parameter is a conversion"
        }
        STRICT_EQUALITY => "a comparison of two unrelated types is checked against the CanEqual instances in scope rather than accepted",
        NO_NONLOCAL_RETURNS => {
            "a return from inside a function literal is an exception thrown through the literal and caught by the method, which every such method pays a try/catch for"
        }
        _ => "",
    }
}

/// `{what} is not allowed under the dialect flag `{flag}`: {cost}`.
pub fn rejected(what: &str, flag: &str) -> String {
    format!("{} is not allowed under the dialect flag `{}`: {}", what, flag, cost(flag))
}

fn strip_comment(line: &str) -> &str {
    let mut quoted = false;
    for (i, c) in line.char_indices() {
        match c {
            '"' => quoted = !quoted,
            '#' if !quoted => return &line[..i],
            _ => {}
        }
    }
    line
}

fn unquote(value: &str) -> Option<&str> {
    value.strip_prefix('"')?.strip_suffix('"')
}
