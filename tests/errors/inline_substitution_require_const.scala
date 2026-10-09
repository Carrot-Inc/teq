// expect: 12:11: error: expected a constant value but found: y + 1
// expect: inlined from tests/errors/inline_substitution_require_const.scala:8
// expect: 1 error found
// `requireConst` of an argument that is no constant reports it as scalac shows it (`y + 1`,
// where the retype path shows the parameter's name), at the call with the line of the argument
// it was inlined from, as scalac's inline stack trace shows them.
import scala.compiletime.requireConst
inline def need(inline x: Int): Int = { requireConst(x); x }
@main def run(): Unit =
  println(need(1 + 2))
  val y = 3
  println(need(y + 1))
