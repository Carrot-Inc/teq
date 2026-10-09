import scala.math.Numeric.Implicits.infixNumericOps
import scala.math.Ordering.Implicits.infixOrderingOps

def show(label: String, values: Any*): Unit = println(label + ": " + values.mkString(" | "))

final case class Counter(n: Int)

given Ordering[Counter] = Ordering.by(_.n)

def largest[T: Ordering](a: T, b: T): T = if a >= b then a else b

def double[T: Numeric](x: T): T = x + x

@main def main(): Unit =
  show("ordering ops", Counter(1) < Counter(2), Counter(1) >= Counter(2), largest(Counter(3), Counter(7)), largest("a", "b"), Counter(5).max(Counter(2)))
  show("numeric ops", double(21), double(1.5) == 3.0, double(4L), -double(2))
