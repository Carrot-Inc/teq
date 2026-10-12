// A wildcard export forwards every member that is neither private nor synthetic (dotty's
// `Namer.addWildcardForwarders`): a user's own `copy` and a `$` in a name are ordinary members, and
// an exported `f(Int)` overloads an inherited `f(String)`, which it does not override (`canForward`).
class Library:
  def copy(n: Int): Int = n + 1
  def cash$value: Int = 7
  def f(x: Int): Int = x + 1
trait Base:
  def f(x: String): Int = x.length
class Public(val library: Library):
  export library.{ copy, cash$value }
class Overloaded(val library: Library) extends Base:
  export library.f
class Everything(val library: Library):
  export library.*

@main def run(): Unit =
  val p = new Public(new Library)
  println(p.copy(2))
  println(p.cash$value)
  val o = new Overloaded(new Library)
  println(o.f(2))
  println(o.f("hi"))
  val e = new Everything(new Library)
  println(e.copy(5) + e.cash$value + e.f(1))
