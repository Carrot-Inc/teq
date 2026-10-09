//! Scala 2 class files: the symbol table scalac 2 pickles into the class file of each top-level
//! class and object is rewritten as a TASTy file holding the same definitions without bodies,
//! which the TASTy loader then reads as it reads any other (`transcode.rs`).

pub mod pickle;
mod transcode;

pub use transcode::{to_tasty, UNSUPPORTED};
