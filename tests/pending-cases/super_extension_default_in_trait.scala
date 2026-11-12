// A trait's extension method selected through `super`
// from a subtrait, its default left out (`h`) and
// given (`k`): scalac 3.8.4 prints the .expected.
// teq is to accept both calls and print it too, in
// every mode (JVM, JavaScript, interpreter, products).
trait T:
  extension (s: Int) def ex(y: Int = 1): Int = s * 10 + y

trait T2 extends T:
  def h: Int = super.ex(5)()
  def k: Int = super.ex(5)(2)

class C extends T2

@main def run(): Unit =
  val c = C()
  println(c.h)
  println(c.k)
