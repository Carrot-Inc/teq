// A right-associative extension with a type parameter whose own first clause is implicit, called written out.
object Lib:
  extension (x: String)
    def %:[A](implicit n: A): String = x + n.toString
@main def run(): Unit = println(Lib.%:[Int](using 7)("x"))
