// A right-associative extension whose own first clause is implicit is swapped, an implicit clause being no using one.
object Lib:
  extension (x: String)
    def %:(implicit n: Int): String = x + n
@main def run(): Unit =
  println(Lib.%:(7)("x"))
