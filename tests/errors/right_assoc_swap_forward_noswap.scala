// A right-associative extension whose own first clause is a using clause is not swapped, so the receiver's type cannot name the method's type parameter.
object Lib:
  extension (x: A)
    def %:[A](using n: Int): String = "ok"
@main def run(): Unit = println("ok")
// expect: 3:17: error: Not found: type A
