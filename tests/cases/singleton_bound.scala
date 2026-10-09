// A type parameter bounded by Singleton takes a literal argument's literal type and a stable
// argument's path type, as under scalac: `Foo(1).t` is a `1` and `Foo(x).t` an `x.type`.
enum Color:
  case Red, Green
case class Foo[T <: Singleton](t: T)
@main def Main(): Unit =
  val a: 1 = Foo(1).t
  val x = 5
  val b: x.type = Foo(x).t
  val c = Foo(Color.Red)
  println((a, b, c))
