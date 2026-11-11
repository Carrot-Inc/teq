// A match type's scrutinee is transformed with `super.transform(selector)` (PostTyper.scala 691-699), which visits
// the selector's parts but not the selector itself, so a scrutinee that is itself an application is not
// bounds-checked (a nested one is: `(C[? >: String], Int) match` is rejected by scalac). scalac 3.8.4 accepts each
// of these; teq agrees since round 22
// (`Worker::with_bounds_unchecked_top`), for `C[String] match` as well.
class C[T <: Int]
type N = C[? >: String] match
  case Any => Int
type M[X] = C[? >: String] match
  case X => Int
def f[X](x: X): C[? >: String] match { case X => Int } = ???
@main def main(): Unit =
  val n: N = 3
  println(n)
