// A method of a generic class created through a path, its argument another creation whose type argument differs
// (`new o.Cell(1, Some(1)).same(new o.Cell(1, None))`): the receiver's type variable is still open when the
// argument is typed, as for a class created plainly.
class Plain[T](val id: Int, val get: T) { def same(o: Plain[T]): Boolean = id == o.id }
class Outer:
  class Cell[T](val id: Int, val get: T) { def same(o: Cell[T]): Boolean = id == o.id }

@main def run(): Unit =
  val o = new Outer
  val c: o.Cell[Option[Int]] = new o.Cell(1, Some(1))
  println(new Plain(1, Some(1)).same(new Plain(1, None)))
  println(s"${c.same(new o.Cell(1, None))} ${new o.Cell(1, Some(1)).same(new o.Cell(1, None))}")
