// A type pattern whose argument is a reference to a value (`D[acc.type]`) is compared with the
// selector's type through the value's type. Where that type holds a variable still open (`acc` is
// a `C[?A]` while the argument of `step` is typed), the comparison bounds it (`?A <: String`, as
// dotty's `constrainPatternType` does for a final pattern class), so the lambda's result `C[Int]`
// is to be rejected as under scalac.
class C[+A]
final class D[+A]
def step[A](initial: A)(f: A => A): A = f(initial)

val result = step(new C[Nothing]) { acc =>
  (new D[C[String]]) match
    case _: D[acc.type] => ()
    case _ => ()
  new C[Int]
}
// expect: 14:3: error: Found:    C[Int]
// expect: Required: C[String]
