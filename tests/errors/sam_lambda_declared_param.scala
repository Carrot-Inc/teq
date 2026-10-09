// expect: 7:37: error: type mismatch: found String => AnyRef, required X[AnyRef]
// expect: 1 error found
// A lambda's declared parameter type must take every argument of the method it implements:
// `X[? >: String <: AnyRef]` is implemented as an `X[AnyRef]`, whose `f` takes any `AnyRef`.
trait X[A] { def f(a: A): A }
@main def main(): Unit =
  val x: X[? >: String <: AnyRef] = (a: String) => a
  val y: X[? >: String <: AnyRef] = (a: AnyRef) => a
  val z: X[String] = (a: String) => a
  println(z.f("z"))
