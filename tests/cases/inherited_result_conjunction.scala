// A member of no written result type that overrides members of two bases takes their results met: AnyVal from A
// and Any from B is AnyVal, so its call is an AnyVal.
trait A { def f(x: Int): AnyVal }
trait B { def f(x: Int): Any }
class C extends A with B { def f(x: Int) = x }
@main def main(): Unit =
  val v: AnyVal = (new C).f(1)
  println(v)
