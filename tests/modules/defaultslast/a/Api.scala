package dlast

// The getters of the last fold across modules. A given whose using clauses each leave parameters
// to their defaults is elaborated one clause at a time (`Typer.adaptNoArgsImplicitMethod`). A
// trait selects a getter on `super` through an accessor of its own (`SuperAccessors`), which a
// downstream class mixing the trait in implements (`ResolveSuper`). An override that takes its
// using parameter's default from the member it overrides calls that member's getter, which the
// split JavaScript keeps although no instance runs the member's body.
def mark(s: String): Int = { print(s); 1 }

class A
class C
class D
class E
class F
class G
class R
object Givens:
  given Unit = ()
  given a(using Unit): A = { print("A"); new A }
  given d(using Unit): D = { print("D"); new D }
  given g(using Unit): G = { print("G"); new G }
  given b(using a: A)(using c: C = { print("C"); new C }, d: D)(using e: E = { print("E"); new E })(using f: F = { print("F"); new F }, g: G): R = { print("R"); new R }
  def use(using r: R): Unit = println("!")

class Base { def f(a: Int = mark("A")): Int = a }
trait Layer extends Base:
  override def f(a: Int = mark("L")): Int = super.f() + a

class X(val n: Int)
trait Parent:
  def go(a: Int)(using x: X = { print("D"); new X(a) }): Int = x.n
class Child extends Parent:
  override def go(a: Int)(using x: X): Int = x.n + a
