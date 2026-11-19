//! The initialization checker's view of the program and its libraries (scalac 3.8.4's `transform/init`, its
//! input): the trees the checker reads, as the reference's `TreeUnpickler` makes them of TASTy, for the program's own
//! pickles and the class path's alike. The checker itself (`Semantic`, `Objects`) is not here yet: this part is the
//! view, held to the reference's own trees by tests/init-view.sh, whose oracle is tests/init-view/InitDump.scala.
//!
//! Its switches, test-only, each an environment variable read where it acts:
//! `TEQ_INIT_VIEW=<root> teq tasty <dir>` prints the view (tasty/cli.rs); `TEQ_INIT_VIEW_CLASSPATH` its class path's jars;
//! `TEQ_INIT_VIEW_MEMBERS=<file>` the library classes whose trees it prints; `TEQ_INIT_VIEW_INVENTORY=<file>`, set
//! for a compile (src/main.rs, beside `TEQ_BODIES_CENSUS`), writes the typed program's classes, which the view then
//! reads to print a class of a withheld body `unavailable`.

pub mod dump;
pub mod graph;
pub mod view;
