// A using parameter whose default a member it overrides declares falls back to that default when
// no given is found, as an explicit clause's omitted argument does (`SymDenotations.hasDefaultParams`,
// `Typer.adaptNoArgsImplicitMethod`).
class X(val n: Int)
class A { def f(using x: X = new X(1)): Int = x.n }
class B extends A { override def f(using x: X): Int = x.n + 1 }
trait P { def p(a: Int)(using x: X = new X(10)): Int = a + x.n }
class Q extends P { override def p(a: Int)(using x: X): Int = a * x.n }
@main def main(): Unit = {
  println(new B().f)
  println((new B: A).f)
  println(new Q().p(3))
  locally {
    given X = new X(7)
    println(new B().f)
  }
}
