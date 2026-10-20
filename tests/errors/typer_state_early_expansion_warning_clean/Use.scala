// The warning of an expansion run ahead of the later phase for a call typed before the one an
// early expansion needs is reported once, in the inlining phase, beside the phase's own error,
// and the argument it was expanded in is reused by the retry on the qualifier's conversion.
// expect: Use.scala:16:11: warning: prefix warning
// expect: Use.scala:19:11: error: after
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
  println(M.fail)
