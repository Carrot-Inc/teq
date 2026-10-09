// jars: scala-library scala2-lib
// A Scala 2.13 trait whose companion object is declared before it (its pickle holds the module class `T` first) keeps
// its private lazy val's holder in the class mixing it in: computed once. The module class was once taken for the
// trait and the value computed at every read.
import companions.T

class C extends T

@main def run(): Unit =
  val c = C()
  println(c.read + c.read)
  println(T.made)
