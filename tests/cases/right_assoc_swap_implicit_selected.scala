// A right-associative extension whose own first clause is implicit, selected on its compiled first argument.
object Lib:
  extension (x: String)
    def %:(implicit n: Int): String = x + n
@main def run(): Unit =
  import Lib.*
  println(7.%:("x"))
