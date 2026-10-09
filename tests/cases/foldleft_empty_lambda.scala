// foldLeft over an empty collection's factory with an untyped lambda: the accumulator's variable is
// instantiated to the factory's type with its element left open, which nothing bounds below and
// dotty's flipBottom maximizes (Typer.typedFunctionValue, inferredFromTarget): `foldLeft(List.empty)` is
// a `List[Any]`, where `Nil`'s improvement gives `List[Int]` (foldleft_nil_lambda). A factory
// applied, `Map()` or `List()`, has its covariant variables instantiated from below as the argument
// is typed (Inferencing.interpolateTypeVars of an `Apply`), its nonvariant ones left open:
// `foldLeft(Map())` is a `Map[Any, Nothing]`, `foldLeft(List())` improved to the body's `List[Int]`.
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
  val m = List(1).foldLeft(Map())((acc, x) => acc)
  val mm: Map[Any, Nothing] = m
  val l = List(1).foldLeft(List())((acc, x) => x :: acc)
  val ll: List[Int] = l
  val s = List(1).foldLeft(Set())((acc, x) => acc)
  val ss: Set[Any] = s
  val v = List(1).foldLeft(Vector())((acc, x) => acc)
  val vv: Vector[Nothing] = v
  println(List(mm, ll, ss, vv))
