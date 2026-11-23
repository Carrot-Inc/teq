//! The native target: Scala Native's libraries and runtime, linked by teq.

pub mod build;
pub mod classpath;
pub mod codegen;
pub mod descriptor;
pub mod generate;
pub mod inflate;
pub mod jar;
pub mod library_options;
pub mod lower;
pub mod meta;
pub mod nir;
pub mod reach;
pub mod scala;
pub mod tools;

/// The target of a link that names none.
pub const DEFAULT_TRIPLE: &str = "x86_64-unknown-linux-musl";
