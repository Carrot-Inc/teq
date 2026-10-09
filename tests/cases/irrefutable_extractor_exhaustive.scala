// teq: --werror
// An extractor typed as a `Some` (`->`) covers a pair of a sealed type as a tuple pattern
// does, so the match needs no other case.
enum Day:
  case Mon, Tue

sealed trait Shape
case class Circle(r: Int) extends Shape
case object Dot extends Shape

def each[A](as: List[A])(f: A => String): String = as.map(f).mkString(",")

@main def run(): Unit =
  println(each(List(Day.Mon -> 1, Day.Tue -> 2)) { case d -> n => s"$d$n" })
  val f: ((Shape, (Day, Boolean))) => String = { case s -> (d -> b) => s"$s $d $b" }
  println(f(Circle(2) -> (Day.Tue -> true)))
  println(Map(Dot -> Day.Mon).map { case k -> v => s"$k=$v" }.mkString)
