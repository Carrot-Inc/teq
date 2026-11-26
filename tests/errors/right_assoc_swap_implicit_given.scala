// A right-associative extension whose own first clause is implicit is swapped, so its selection on the receiver takes no given.
object Lib:
  extension (x: String)
    def %:(implicit n: Int): String = x + n
given Int = 7
@main def run(): Unit =
  import Lib.*
  println("x".%:)
// expect: 8:15: error: value %: is not a member of String.
