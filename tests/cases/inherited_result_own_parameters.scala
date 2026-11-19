// A member of no written result type that overrides takes the overridden's result with the overridden's value
// parameters read as its own: a singleton of a parameter, a type selected from one, a parameter in a later clause.
trait B {
  def f(x: Any): Option[x.type]
  def k(x: Any): x.type
  def m(x: String): Seq[x.type]
  trait Ctx { type T; def v: T }
  def g(c: Ctx)(x: c.T): c.T
}
class D extends B {
  def f(y: Any) = Option(y)
  def k(y: Any) = y
  def m(x: String) = List(x)
  def g(d: Ctx)(y: d.T) = y
}
@main def main(): Unit =
  val d = new D
  val ctx = new d.Ctx { type T = Int; def v = 7 }
  println(List(d.f(1), d.k("a"), d.m("b"), d.g(ctx)(ctx.v)).mkString(" "))
