// A written `apply` matching the synthesized one loosely (as many type parameters, every clause's
// parameter types the same) retracts it (`Namer.invalidateIfClashingSynthetic`), so its defaults
// are the only ones: no "two or more overloaded variants" for a generic or a curried class.
case class C[A](x: Int = 1)
object C:
  def apply[A](x: Int = 2): C[A] = new C[A](x)
case class D(x: Int = 1)(val y: Int)
object D:
  def apply(x: Int = 2)(y: Int): D = new D(x)(y)

@main def run(): Unit =
  println(C[String]())
  println(D()(3).x)
