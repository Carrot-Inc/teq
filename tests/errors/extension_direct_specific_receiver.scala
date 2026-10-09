// expect: 14:25: error: type mismatch: found String, required Int
// expect: 15:30: error: type mismatch: found String, required Int
// expect: 17:29: error: type mismatch: found String, required Int
// A receiver more specific than every other is taken whatever the argument lists after it take,
// when the call names the extension directly as when it selects it, though the generic
// overload would take them (scalac's first-list rule).
object A:
  extension (s: String)(using x: Int) def g(y: Int): Int = 1
  extension [T](s: T)(using x: Int) def g(y: String): Int = 2
  extension (s: String)(using x: Int) def h(y: Int, z: Int): Int = 1
  extension [T](s: T)(using x: Int) def h(y: String): Int = 2
object Main:
  given Int = 3
  def direct = A.g("a")("b")
  def directArity = A.h("a")("b")
  import A.*
  def selectedArity = "a".h("b")
