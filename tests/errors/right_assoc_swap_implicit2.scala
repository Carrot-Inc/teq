// An implicit clause before another clause of a right-associative extension.
object Lib:
  extension (x: String)
    def %:(implicit n: Int)(y: String): String = x + n + y
@main def run(): Unit =
  println(Lib.%:(7)("x")("y"))
// expect: 4:28: error: '=' expected, but '(' found
// expect: 6:20: error: method %: in object Lib does not take more parameters
