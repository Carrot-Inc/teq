// Three runs under scalac. `Marks.noted` is a macro of this build and `Raised.raised` a transparent
// macro whose implementation calls it: Raised.scala waits for a second run, whose inlining phase
// expands that call and reports the macro's warning; this file calls the transparent macro, so it
// is typed in a third run, whose typer reports the type error. What the second run's phase
// reported stays reported beside it.
// expect: Raised.scala:6:19: warning: noted by the macro
// expect: Main.scala:10:21: error: type mismatch
// expect: 1 error found
def lifted: Int = Raised.raised(9)
val wrong: String = 3
