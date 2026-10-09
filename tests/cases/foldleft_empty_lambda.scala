// foldLeft over an empty collection's factory with an untyped lambda: the accumulator's variable is
// instantiated to the factory's type with its element left open, which nothing bounds below and
// dotty's flipBottom maximizes (Typer.typedFunctionValue, inferredFromTarget): `foldLeft(List.empty)` is
// a `List[Any]`, where `Nil`'s improvement gives `List[Int]` (foldleft_nil_lambda).
def fold[B](z: B)(op: (B, Int) => B): B = op(z, 1)
@main def run() =
  val a = List(1, 2).foldLeft(List.empty)((acc, x) => x :: acc)
  println(a)
  val b = fold(Vector.empty)((acc, x) => acc :+ x)
  println(b)
  val c = List(1).foldLeft(Option.empty)((acc, x) => acc.orElse(Some(x)))
  println(c)
  val d = List(3).foldLeft(Map.empty)((acc, x) => acc + (x -> "a"))
  println(d)
  val e = List(4).foldLeft(Set.empty)((acc, x) => acc + x)
  println(e)
  val f: List[Int] = List(1, 2).foldLeft(List.empty)((acc, x) => x :: acc)
  println(f.sum)
  val g = List(1, 2).foldLeft(Nil)((acc, x) => x :: acc)
  println(g.sum)
  val h = List(5).foldLeft(List.empty)((acc, x) => acc)
  println(h)
  val aa: List[Any] = a
  val bb: Vector[Any] = b
  val cc: Option[Any] = c
  println(aa.length + bb.length + cc.size)
