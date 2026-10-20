// The inlining phase runs only after a typer that reported no error (`Phase.isRunnable`): a plain
// `compiletime.error` a transparent expansion meets waits for it, so B's typing error is the build's one
// (before, the typer reported A's beside it).
// expect: B.scala:5:16: error: type mismatch
// expect: 1 error found
// absent: A_EARLY
transparent inline def boom: Int = scala.compiletime.error("A_EARLY")
val early: Int = boom
