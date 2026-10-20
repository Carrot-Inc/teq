// The warning of an expansion run ahead of the inlining phase for a call typed before the one an
// early expansion needs is the phase's, not run after a typer error (`_clean` warns), and the
// argument is reused by the retry on the qualifier's conversion: the one error is the last line's.
// absent: prefix warning
// expect: Use.scala:19:21: error: type mismatch
// expect: 1 error found
import scala.language.implicitConversions

class W:
  def m(x: String): String = "W " + x
class V:
  def m(x: Int): String = "V " + x
given Conversion[W, V] = _ => V()

@main def run(): Unit =
  val a = M.warn
  println(W().m(M.id(M.one)))
  println(a)
  val bad: String = a
