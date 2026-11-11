// Overloaded private members an inline method reads: each alternative has an accessor of its own, the two
// entered in the class under the one name (`newAccessorSymbol`'s `entered`), so each call reaches its member.
class O:
  private def f(x: Int): Int = x + 1
  private def f(x: String): Int = x.length
  inline def all: String = s"${f(1)} ${f("abc")}"
  inline def swapped: String = s"${f("ab")} ${f(4)}"
object P:
  private def g(x: Int): String = s"int $x"
  private def g(x: Boolean): String = s"bool $x"
  inline def both: String = g(7) + ", " + g(true)
@main def run(): Unit =
  println(new O().all)
  println(new O().swapped)
  println(P.both)
