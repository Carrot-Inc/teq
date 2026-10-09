// expect: 8:15: error: type mismatch: found Int, required T
// expect: 9:15: error: type mismatch: found Int, required T
// expect: 2 errors found
// A var or a computed value is no singleton: scalac's `Found: (y : Int), Required: Singleton`.
case class Foo[T <: Singleton](t: T)
@main def Main(): Unit =
  var y = 1
  println(Foo(y))
  println(Foo(y + 1))
