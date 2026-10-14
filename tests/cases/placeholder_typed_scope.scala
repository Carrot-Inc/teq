// A placeholder alone, ascribed, annotated or in parentheses, belongs to the enclosing expression:
// `(_: Score).map(f)` is one function, its parameter typed `Score` (dotty's `Parsers.isWildcard`).
object Report:
  opaque type Score = Double
  object Score:
    def apply(d: Double): Score = d
    extension (s: Score) def map(f: Double => Double): Score = f(s)
    extension (s: Score) def value: Double = s

def create(score: Report.Score => Report.Score = identity): Report.Score = score(Report.Score(1.25))
def show(i: Int): String = s"<$i>"
val critical = true

@main def run(): Unit =
  println(create(score = (_: Report.Score).map(_ * (if critical then 2 else 1))).value)
  println(List(1).map((_: Int) + 1))
  println(List(1).map((_: Int).toString + "!"))
  val f = (_: Int).toString
  println(f(3))
  println(List(2).map((_: Int @unchecked) * 2))
  println(List(3).map(((_: Int)) * 3))
  val g = (_: Int, _: String)
  println(g(1, "a"))
  println(List(4).map(show(_: Int)))
  println(List(5, 6).foldLeft(0)(_ + (_: Int)))
  val h = List(7).map(x => (_: Int) + x)
  println(h.map(_(1)))
