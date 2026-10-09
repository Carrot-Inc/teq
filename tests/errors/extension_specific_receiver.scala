// expect: 13:11: error: type mismatch: found String, required Int
// expect: 14:11: error: type mismatch: found String, required Int
// The argument lists after the receiver decide only between equally specific receivers: a
// `String` receiver more specific than every other is taken, and its argument is then wrong,
// though the generic overload would take it (scalac's first-list rule).
object A:
  extension (s: String)(using x: Int) def g(y: Int): Int = 1
  extension [T](s: T)(using x: Int) def g(y: String): Int = 2
object Main:
  def main(args: Array[String]): Unit =
    import A.*
    given Int = 3
    "a".g("b")
    "a".h("b")
extension (s: String) def h(y: Int): Int = 1
extension [T](s: T) def h(y: String): Int = 2
