// A plain `if` of a transparent method whose condition is under a cast to `Boolean` is not
// reduced by scalac's typer and keeps both branches: the other branch's `compiletime.error` is
// reported by the inlining phase, at the call.
// expect: 12:13: error: the other branch is kept
object Lib:
  inline def yes: Boolean = true

object Gate:
  transparent inline def plainIf_cast(inline b: Boolean): Int =
    if b.asInstanceOf[Boolean] then 100 else scala.compiletime.error("the other branch is kept")

val cell0 = Gate.plainIf_cast((true: Boolean)) // plainIf, under a cast to Boolean
