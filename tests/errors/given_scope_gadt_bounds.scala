// The candidates of a type's implicit scope are matched under the run's context (`OfTypeImplicits.eligible`,
// Implicits.scala:285-290, made with `runContext`, 776): a case's GADT bound `A = Int` does not make the
// companion's `Show[Int]` eligible for `Show[A]` (scalac 3.8.4 refuses the search).
// expect: error: no given instance of type Show[A] was found for parameter x
trait Show[A] { def show: String }
object Show {
  given Show[Int] with { def show = "int" }
}
enum E[A] {
  case I extends E[Int]
}
def f[A](e: E[A]): String = e match {
  case E.I => summon[Show[A]].show
}
@main def main(): Unit = println(f(E.I))
