// The expansion of `M.warn` reports its warning
// outside the argument's typing: scalac retries the
// member's application of `m` on the conversion
// of its qualifier, reusing the argument (`V 1`).
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
