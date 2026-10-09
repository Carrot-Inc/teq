// expect: right_assoc_extension_calls.scala:16:21: error: type mismatch: found Int, required String
// expect: right_assoc_extension_calls.scala:12:9: error: right-associative extension method must start with a single parameter, consider a tupled parameter instead
// expect: right_assoc_extension_calls.scala:17:11: error: Too many type arguments for Syntax.*:[A]
// A right-associative extension method called as a method takes the method's first clause
// first (dotty's `Desugar.extMethod`, `rightAssocParams`), which has to be a single parameter; its
// explicit type arguments fill the extension's type clause alone, `[A]` of `[A][B]` (E023).
// teq reports the receiver written second as well.
object Syntax:
  extension (n: Int)
    def +:(s: String): String = s + n
  extension (n: Int)
    def -:(a: String, b: String): String = a + b + n
  extension [A](n: A)
    def *:[B](s: B): String = s.toString + ":" + n.toString
@main def run(): Unit =
  println(Syntax.+:(2)("x"))
  println(Syntax.*:[Int, String]("x")(2))
