// A given resolved for another given's using clause whose own using parameter falls back to its
// default: the clause applied again with the given found and the default left out
// (`Typer.adaptNoArgsImplicitMethod`), whose plan lifts the earlier argument first (`liftFun`), so
// the effectful given runs once.
class A(val n: Int)
class B(val n: Int)
class C(val n: Int)
var count = 0
given Unit = ()
given mkA(using Unit): A = { count += 1; print("A"); new A(count) }
given mkB(using a: A)(using c: C = { print("C"); new C(a.n) }): B = { print("B"); new B(a.n * 10 + c.n) }
def use(using b: B): Unit = println(b.n)
@main def main(): Unit =
  use
  use
