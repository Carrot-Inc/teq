// A given's condition in parentheses may be types alone (dotty's `GivenConditional ::= '(' FunArgTypes ')'`), whose
// parameters `termParamClause` names `x$1`, `x$2` (`contextTypes`), anonymous and named alike. scalac 3.8.4 prints
// `(int,str)` and `3`.
trait Semigroup[A] { def name: String }
given Semigroup[Int] with { def name = "int" }
given Semigroup[String] with { def name = "str" }
given [A, B] => (Semigroup[A], Semigroup[B]) => Semigroup[(A, B)] =
  new Semigroup[(A, B)] { def name = s"(${summon[Semigroup[A]].name},${summon[Semigroup[B]].name})" }
class Generic; class Context
class Show[T](val v: Int)
given Generic = Generic()
given Context = Context()
given showGen: [T] => (Generic, Context) => Show[T] = new Show[T](3)
@main def run(): Unit =
  println(summon[Semigroup[(Int, String)]].name)
  println(summon[Show[Int]].v)
