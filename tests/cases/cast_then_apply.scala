// `x.asInstanceOf[T](args)` is the cast applied to the arguments, `asInstanceOf[T]` being a
// parameterless method.
@main def run(): Unit =
  val any: Any = (x: Int) => x + 1
  println(any.asInstanceOf[Int => Int](41))
  val table: Any = Map("a" -> 1)
  println(table.asInstanceOf[Map[String, Int]]("a"))
  val seq: AnyRef = Vector(5, 6, 7)
  println(seq.asInstanceOf[Vector[Int]](2))
