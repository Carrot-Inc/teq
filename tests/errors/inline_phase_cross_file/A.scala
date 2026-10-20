// The inlining phase runs only after a typer that reported no error, in any file (dotty's run gates a
// phase on its reporter, `Phase.isRunnable`): A's expansion would report its `error`, B's typing
// error is the build's one.
// expect: B.scala:6:19: error: type mismatch
// expect: 1 error found
// absent: expansion must wait
import scala.compiletime.error
inline def fail: Int = error("expansion must wait")
val a: Int = fail
