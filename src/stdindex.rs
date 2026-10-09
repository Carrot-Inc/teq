//! The std index that `build.rs` generates from the files under `std/`: what each file defines
//! at the top level, by package, so that a file is entered when a lookup first asks for one of
//! its names (`typer/stdlib.rs`).

pub(crate) struct StdFileIndex {
    /// The path the file is known by in diagnostics, `<std>/...`.
    pub path: &'static str,
    /// How many `package p:` blocks the file holds, each an AST and a file slot of its own.
    pub blocks: u32,
    /// The packages the file has members in.
    pub packages: &'static [&'static str],
    /// The top-level definitions, as (package path, name, what a lookup asks for it as: `TYPE`,
    /// `TERM` or both): classes, objects, traits, enums, type aliases, vals, defs, named givens,
    /// extension methods and what its exports bring in.
    pub defines: &'static [(&'static str, &'static str, u8)],
    /// The parents its classes, objects and enums extend, as (package, name) resolved among the
    /// std's own definitions, or ("", name) where the head resolves to none: the file enters
    /// when such a class completes, so that the class knows its subclasses.
    pub extends: &'static [(&'static str, &'static str)],
    /// The packages where the file holds package-level givens or implicit definitions, which
    /// a search finds by type rather than by name: they enter with the package's given index.
    pub unnamed_givens: &'static [&'static str],
    /// The definitions marked `@predef`: they stand for members of scala-library's `Predef`,
    /// which an explicit import of `Predef` can hide.
    pub predef: &'static [&'static str],
}

pub(crate) const TYPE: u8 = 1;
pub(crate) const TERM: u8 = 2;

// `STD_INDEX`, and `STD_IDENTITY`: a hash of every std file's path and text, which names the
// std's documents in the language server (`typer/loader/attach.rs`).
include!(concat!(env!("OUT_DIR"), "/std_index.rs"));

/// The index entry of a std file by its path.
pub(crate) fn std_file_index(path: &str) -> Option<&'static StdFileIndex> {
    STD_INDEX.iter().find(|e| e.path == path)
}
