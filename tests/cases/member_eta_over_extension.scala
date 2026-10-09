// With a function expected, a member that needs arguments is eta-expanded rather than losing to
// a parameterless extension of the same name that every value has.

extension [A](self: A | Unit) def get: A = self.asInstanceOf[A]

@main def main(): Unit =
  val byType = Map(1 -> List("a"), 2 -> List("b", "c"))
  val o: Option[Int] = Some(2)
  println(o.flatMap(byType.get))
  println(List(1, 3).flatMap(byType.get))
  val present: Int | Unit = 5
  println(present.get)
  println(List(1, 2, 3).mkString)
