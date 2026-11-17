// The own `==` and `!=` of a generic class named through a path where an operand's type argument is inferred from
// the other operand (`c == new o.Cell(1, None)` for a `c: o.Cell[Option[Int]]`): the created class's type
// variable is still open when the operand is typed, as it is for a class created plainly.
class Outer:
  class Cell[T](val id: Int, val get: T) { def ==(o: Cell[T]): Boolean = id == o.id; def !=(o: Cell[T]): Boolean = id != o.id }
  case class Wrap[T](get: T) { def ==(o: Wrap[T]): Boolean = true }

@main def run(): Unit =
  val o = new Outer
  val c: o.Cell[Option[Int]] = new o.Cell(1, Some(1))
  val w: o.Wrap[List[Int]] = o.Wrap(List(1))
  println(s"${c == new o.Cell(1, None)} ${c != new o.Cell(1, None)}")
  println(w == o.Wrap(Nil))
  println(new o.Cell(1, Some(1)) == new o.Cell(1, None))
