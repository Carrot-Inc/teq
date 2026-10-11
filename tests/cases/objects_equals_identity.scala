// java.util.Objects.equals as the JDK's: the same object is equal before its `equals` is asked,
// another asks it; a boxed Int is no boxed Long, where Scala's `==` says they are.
class Loud:
  override def equals(o: Any): Boolean = { println("equals asked"); true }

@main def run(): Unit =
  val loud = new Loud
  println(java.util.Objects.equals(loud, loud))
  println(java.util.Objects.equals(loud, "x"))
  val i: Any = 1
  val l: Any = 1L
  println(java.util.Objects.equals(i, l))
  println(i == l)
  println(java.util.Objects.equals(0.0, -0.0))
