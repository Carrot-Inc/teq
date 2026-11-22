//! The native target: Scala Native's libraries and runtime, linked by teq.

pub mod classpath;
pub mod codegen;
pub mod generate;
pub mod lower;
pub mod meta;
pub mod nir;
pub mod reach;
pub mod scala;

/// The target of a link that names none.
pub const DEFAULT_TRIPLE: &str = "x86_64-unknown-linux-musl";
