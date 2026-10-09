case class Leaf[T](v: T)
def pick[U](xs: List[Leaf[?]])(f: PartialFunction[Leaf[?], U]): List[U] = xs.collect(f)

@main def run(): Unit =
  val xs: List[Leaf[?]] = List(Leaf(1), Leaf("a"))
  val ys: List[Leaf[Int]] = pick(xs) { case l: Leaf[Int] @unchecked => l }
  println(ys.length)
  val strings: List[String] = pick(xs) { case Leaf(s: String) => s }
  println(strings)
