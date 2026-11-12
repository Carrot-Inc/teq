// A type pattern of a class defined in a block whose parent names a value of the block (`L extends
// D[acc.type]`) is compared with the selector's type through that parent. Where the value's type
// holds a variable still open (`acc` is a `C[?A]` while the argument of `step` is typed), the
// comparison bounds it (`?A <: String`), so the lambda's result `C[Int]`
// is to be rejected as under scalac.
class C[+A]
class D[+A]
def step[A](initial: A)(f: A => A): A = f(initial)

val result = step(new C[Nothing]) { acc =>
  final class L extends D[acc.type]
  (new D[C[String]]: D[C[String]]) match
    case _: L => ()
    case _ => ()
  new C[Int]
}
// expect: 15:3: error: Found:    C[Int]
// expect: Required: C[String]
