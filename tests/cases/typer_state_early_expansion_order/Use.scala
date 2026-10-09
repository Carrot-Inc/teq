// A pending plain call expanded before the later phase, for a constant narrowing or as an argument
// a transparent expansion copies, expands after the pending calls typed before it, in their order:
// each `wide` expands before the `one` after it, as in scalac, also
// where the expansion runs inside an application given up for a conversion of its qualifier,
// whose retraction keeps the expansion of a call typed before it.
import scala.language.implicitConversions

class W:
  def m(x: String): String = "W " + x
class V:
  def m(x: Int): String = "V " + x
given Conversion[W, V] = _ => V()

@main def run(): Unit =
  val a = A.wide
  val b: Byte = A.one
  println(s"$a $b")
  val c = B.wide
  println(T.twice(B.one) + c)
  val e = C.wide
  println(W().m(T.twice(C.one)))
  println(e)
