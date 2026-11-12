// jars: view-lib
// A body that names the library's match alias `K.Lift`,
// then a library body that names it: the alias seen
// through `K` is one alias for both. The signature
// below names `K`.
import viewlib.*

def loads(k: K.type): Unit = ()

@main def main(): Unit =
  val lift: K.Lift[Int] = "own"
  println(lift)
  println(Use.lifted)
