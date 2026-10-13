// An anonymous instance is refined by each declaration whose information is strictly narrower than the
// inherited member's it matches (dotty's `TypeOps.classBound`, `decl.matches` then the full-info comparison): a
// polymorphic method over its renamed type parameters, and each alternative of an overloaded one by the
// inherited alternative of its parameters, which a call then selects with the narrower result.
trait P:
  def f[A](a: A): Any
trait Q:
  def g(a: Int): Any
  def g(a: String): Any

@main def main(): Unit =
  val x = new P { def f[A](a: A): String = "poly " + a }
  val y: String = x.f(1)
  println(y)
  val z = new Q { def g(a: Int): String = "int " + a; def g(a: String): String = "string " + a }
  val i: String = z.g(1)
  val s: String = z.g("s")
  println(i)
  println(s)
