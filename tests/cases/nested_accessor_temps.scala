// An object's accessor bound in a body and read inside a closure of it that binds another
// object's accessor whose temporary would take the same name (`Nil` and `None`).
def table(xs: List[Int]): List[List[Any]] =
  val head: List[Any] = List(Nil, Nil, Nil, Nil)
  val row = (x: Int) =>
    val empty: List[Int] = Nil
    val nones: List[Option[Int]] = List(None, None, None, None, None, None, None, None)
    val picked: Option[Int] = if x > 1 then Some(x) else None
    empty :: picked :: nones
  val tail: List[Any] = List(Nil, Nil, Nil, Nil)
  head :: xs.map(row) ::: List(tail)

@main def run(): Unit =
  table(List(1, 2)).foreach(println)
