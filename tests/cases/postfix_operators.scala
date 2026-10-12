import scala.language.postfixOps

// Postfix operators where the language import enables them (dotc's infixOps under
// postfixOpsEnabled): an operator with no operand after it selects the member of the whole infix
// expression before it, and a builtin operator so selected where a function is expected is
// eta-expanded, as lila's `from.takeWhile('-' !=)` and `filter(0 <)`.
object Rating:
  def parse(from: String): Option[(String, String)] =
    for
      min <- Option(from.takeWhile('-' !=))
      max <- Option(from.dropWhile('-' !=).drop(1))
    yield (min, max)

@main def run(): Unit =
  println(Rating.parse("1200-1500"))
  val xs = List(3, -1, 4, 0)
  println(xs.filter(0 <))
  println(xs.map(1 +).sum)
  println(xs.exists(4 ==))
  val n = xs.length + 1 toString;
  println(n)
  println(List(1, 2) reverse)
  val s = "a" + "b" length;
  println(s)
