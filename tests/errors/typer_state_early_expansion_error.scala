// expect: 20:22: error: prefix error
// expect: 1 error found
// The error of a pending call's expansion run ahead of the later phase, where a transparent method
// copies it, is reported at the flush of its unit, no part of the typing of the argument: the
// member's application of `m` is retried on its qualifier's conversion, which applies, and the
// expansion's error alone is reported, as scalac and master report it.
import scala.language.implicitConversions

object M:
  inline def bad: Int = scala.compiletime.error("prefix error")
  transparent inline def id(inline x: Int): Int = x

class W:
  def m(x: String): String = "W " + x
class V:
  def m(x: Int): String = "V " + x
given Conversion[W, V] = _ => V()

@main def run(): Unit =
  println(W().m(M.id(M.bad)))
