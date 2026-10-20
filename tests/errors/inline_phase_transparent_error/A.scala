// A plain `compiletime.error` a transparent expansion meets stays a call for the inlining phase (dotty's
// `Inlines.needsInlining` in the typer is `needsTransparentInlining`): the phase reports it at the outermost
// call, and B's plain `error` beside it (before, the typer reported A's, and B's never ran).
// expect: A.scala:8:18: error: A_EARLY
// expect: B.scala:5:16: error: B_ERROR
// expect: 2 errors found
transparent inline def boom: Int = scala.compiletime.error("A_EARLY")
val early: Int = boom
