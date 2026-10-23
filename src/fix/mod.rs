//! `teq fix` (docs/FIX.md): lint and rewrite rules, written in Scala against the std's `teq.fix` API and run by
//! the interpreter, over the files the compiler parsed and typed in the same process.

pub mod cli;
pub mod config;
pub mod engine;
pub mod escape;
pub mod host;
pub mod nodes;
pub mod plan;
pub mod service;
pub mod source;
