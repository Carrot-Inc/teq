// expect: 15:25: error: reference to k is ambiguous: it is both defined in package <empty> and imported subsequently by import source.*
// expect: 1 error found
// A local inline method called before its definition, an import from a val defined between the
// two in the block, and a definition of the same name in an enclosing scope: the body is checked
// with the import entered, as the block will enter it, where its `k` is ambiguous (scalac 3.8.4's
// E049, "Reference to k is ambiguous. It is both defined in package <empty> and imported
// subsequently by import (source : Box)._"), reported once at the definition.
class Box:
  val k: Int = 2
def k: Int = 99
def host(): Int =
  val first = g()
  val source = new Box
  import source.*
  inline def g(): Int = k
  first
@main def main(): Unit = println(host())
