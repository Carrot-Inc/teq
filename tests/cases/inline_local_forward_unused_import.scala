// A local inline method called before its definition, an import from a val defined between the
// two in the block: scalac 3.8.4's E039 is for a body that reads the import's binding; one that
// reads nothing of it (a named selector it does not name, a wildcard whose members it names
// none of, binding its own) expands as any other.
class Box:
  val k: Int = 2
  val m: Int = 3
def named(): Int =
  val first = g()
  val source = new Box
  import source.k
  inline def g(): Int = 42
  first
def wildcard(): Int =
  val first = g()
  val source = new Box
  import source.*
  inline def g(): Int = { val y = 40; y + 2 }
  first
def other(): Int =
  val first = g(7)
  val source = new Box
  import source.k
  import source.m
  inline def g(x: Int): Int = x
  first
@main def main(): Unit =
  println(named())
  println(wildcard())
  println(other())
