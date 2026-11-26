// A right-associative extension whose own first clause is a using clause of the receiver's name.
object Lib:
  extension (x: Int)
    def %:(using x: String): String = x
@main def run(): Unit = println(Lib.%:(1)(using "ok"))
// expect: 4:18: error: x is already defined as parameter x
// expect: 4:39: error: Found:    (x : Int)
