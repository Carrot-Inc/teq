// Where scalac 3.8.4 is wrong: a curried overload's later clause is resolved on copies of the alternatives
// (Applications.scala 2798-2822, resolveMapped), which keep the original's cached flags but find no overridden member
// under their trimmed signatures, so an override inherits its default there only where Checking.scala 1383's
// short-circuiting `decl.hasDefaultParams && other.hasDefaultParams` cached it, which follows declaration order.
// B1 declares the inheriting override first: scalac refuses, 15:11: Ambiguous overload. The overloaded alternatives
// of method m in class B1 with types (E051), and 16:11: None of the overloaded alternatives of method m in class
// B1 with types (E134). B2 declares it second: scalac prints 27 and 11. teq reads it in either order: 27, 11, 27, 11.
object Ord {
  trait X; trait Y; object Z extends X with Y
  class A { def m(a: X)(b: Int = 1): Int = b + 0 }
  class B1 extends A { override def m(a: X)(b: Int): Int = b + 10; def m(a: Y)(b: Int): Int = b + 20 }
  class B2 extends A { def m(a: Y)(b: Int): Int = b + 20; override def m(a: X)(b: Int): Int = b + 10 }
}
@main def run(): Unit =
  println(new Ord.B1().m(Ord.Z)(7))
  println(new Ord.B1().m(Ord.Z)())
  println(new Ord.B2().m(Ord.Z)(7))
  println(new Ord.B2().m(Ord.Z)())
