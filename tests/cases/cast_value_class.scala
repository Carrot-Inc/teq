// A cast to a value class tests the value as the class's box before its field is read, where
// a plain `Int` fails (`Erasure.Boxing.unbox` adapts the value to the class).
class V(val i: Int) extends AnyVal

@main def run(): Unit =
  val a: Any = new V(7)
  println(a.asInstanceOf[V].i)
  val b: Any = 7
  try println(b.asInstanceOf[V].i)
  catch case e: ClassCastException => println(e.getMessage.takeWhile(_ != '(').trim)
