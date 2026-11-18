//! The initialization checker's view of the program and its libraries (scalac 3.8.4's `transform/init`, its
//! input): the trees the checker reads, as the reference's `TreeUnpickler` makes them of TASTy, for the program's own
//! pickles and the class path's alike. The checker itself (`Semantic`, `Objects`) is not here yet: this part is the
//! view, held to the reference's own trees by tests/init-view.sh, whose oracle is tests/init-view/InitDump.scala.

pub mod dump;
pub mod view;
