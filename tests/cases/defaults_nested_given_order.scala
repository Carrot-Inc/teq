// A given whose using clauses each leave some parameters to their defaults: every clause is applied
// again in its turn with the givens found (`Typer.adaptNoArgsImplicitMethod`, one using clause at a
// time), so a clause written out of order lifts its own arguments, never a later clause's, before the
// later clause's defaults.
class A
class C
class D
class E
class F
class G
class R
object Api {
  given Unit = ()
  given a(using Unit): A = { print("A"); new A }
  given d(using Unit): D = { print("D"); new D }
  given g(using Unit): G = { print("G"); new G }
  given b(using a: A)(using c: C = { print("C"); new C }, d: D)(using e: E = { print("E"); new E })(using f: F = { print("F"); new F }, g: G): R = { print("R"); new R }
  def use(using r: R): Unit = println("!")
}
@main def main(): Unit = { import Api.given; Api.use }
