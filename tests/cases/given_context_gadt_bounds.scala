// A context's candidates are matched under the search's own context, its GADT bounds included (`searchImplicit`,
// Implicits.scala:1704-1708: `uncachedEligible` while the GADT state narrows), where the implicit scope's are matched
// under the run's (`OfTypeImplicits.eligible`, tests/errors/given_scope_gadt_bounds.scala): `local` serves `Show[A]`
// under `case E.I` (A = Int), as scalac 3.8.4 has it.
trait Show[A] { def show: String }
object Show {
  given Show[Int] with { def show = "int" }
}
enum E[A] {
  case I extends E[Int]
}
given local: Show[Int] with { def show = "local int" }
def f[A](e: E[A]): String = e match {
  case E.I => summon[Show[A]].show
}
@main def main(): Unit = println(f(E.I))
