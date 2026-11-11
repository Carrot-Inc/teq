// A trait's extension method selected through `super` from a subtrait, its default left out (`h`) and given (`k`):
// scalac 3.8.4 prints the .expected. teq refuses both calls in every mode (JVM, JavaScript, interpreter, products):
// "value ex is not a member of the parents of trait T2" (5:16 and 6:16). The selection on `super` does not find the
// parent's extension method. Cause not located. Found by reviewer #13 of the workers exports helper (push 8); the same
// on integration b602956824 and before.
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
