// A member that cannot take its argument retries on the qualifier: a lexical extension whose
// using clause before its parameters has no instance fails its prefix `f(qual)`, and the
// conversion of the receiver is searched, as dotty's `tryExtensionOrConversion` searches it
// after a failed `tryExtension` (scalac `converted s`; teq kept the member's error before).
import scala.language.implicitConversions
class X
class C:
  def f(a: Int): String = "member"
class D:
  def f(a: String): String = "converted " + a
given Conversion[C, D] = _ => D()
extension (c: C)(using x: X) def f(a: String): String = "lexical " + a
@main def run(): Unit =
  println(C().f("s"))
