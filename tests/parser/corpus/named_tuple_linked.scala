// Named tuples against scala-library's `NamedTuple`, which a build under
// `--std=scala-library` finds on the class path, as against the lean std's.
def stats(xs: List[Int]): (count: Int, total: Int) = (count = xs.size, total = xs.sum)

@main def run(): Unit =
  val s = stats(List(1, 2, 3))
  println(s.count + s.total)
  val (c, t) = s
  println(s"$c $t")
  println(s.toTuple)
