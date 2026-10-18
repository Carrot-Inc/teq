// Type-level addition over unary numbers, each request's result the open variable `R` the next request narrows
// (`Add[Succ[A], B, Succ[R]]` asks for `Add[A, B, R]`): the requests shrink, an open variable counting as the larger
// of its two sides' bounds (`Types.TypeSizeAccumulator`), so none diverges, as in scalac, where the sum of every
// bound reached through the variables grew with each request.
sealed trait Num { def value: Int }
final case class Succ[N <: Num](pred: N) extends Num { def value = pred.value + 1 }
final class Zero extends Num { def value = 0 }
val zero = new Zero

trait Add[A <: Num, B <: Num, R <: Num] { def apply(a: A, b: B): R }
object Add:
  given none[B <: Num]: Add[Zero, B, B] = new Add[Zero, B, B] { def apply(a: Zero, b: B) = b }
  given more[A <: Num, B <: Num, R <: Num](using inner: Add[A, B, R]): Add[Succ[A], B, Succ[R]] =
    new Add[Succ[A], B, Succ[R]] { def apply(a: Succ[A], b: B) = Succ(inner(a.pred, b)) }

def add[A <: Num, B <: Num, R <: Num](a: A, b: B)(using ev: Add[A, B, R]): R = ev(a, b)

@main def run(): Unit =
  val three = add(Succ(Succ(zero)), Succ(zero))
  val exact: Succ[Succ[Succ[Zero]]] = three
  println(exact.value)
  val five = add(Succ(Succ(Succ(zero))), Succ(Succ(zero)))
  println(five.value)
