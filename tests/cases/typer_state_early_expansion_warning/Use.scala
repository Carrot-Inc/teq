// The expansion of `M.warn`, typed before `M.one` and expanded with it ahead of the later phase
// where `M.id` copies it, reports its warning at the flush, outside the
// argument's typing: the member's application of `m` is retried on the conversion of its
// qualifier, reusing the argument, as scalac does (`V 1`).
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
