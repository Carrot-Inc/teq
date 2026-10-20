// lila: TutorPerfReport: ClockPercent.from(globalClock.so(_.valueMetric(user.perfType)))
// B of `pick` is instantiated to the lambda's result type as written, the alias
// BothOption[Double]; scalac keeps the alias, so the higher-kinded M[_] of the first `from`
// unifies with BothOption (M[Double] >: BothOption[Double]). That alternative wins and the
// SameRuntime alternative is never needed.
trait SameRuntime[A, T] { def apply(a: A): T }
object ClockPercent:
  def from[M[_]](f: M[Double]): M[String] = f.asInstanceOf[M[String]]
  def from[M[_], B](using SameRuntime[B, Double])(f: M[B]): M[String] = f.asInstanceOf[M[String]]

case class Both[A](white: A, black: A)
type BothOption[A] = Option[Both[A]]
def valueMetric(i: Int): BothOption[Double] = Some(Both(i * 1.0, i * 2.0))
def pick[B](f: Int => B): B = f(2)

@main def run() =
  val clock = pick(i => valueMetric(i))
  val pct: BothOption[String] = ClockPercent.from(clock)
  println(pct)
