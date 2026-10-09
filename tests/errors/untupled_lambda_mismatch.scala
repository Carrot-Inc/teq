// expect: 10:27: error: type mismatch: found (<error>, <error>, <error>) => <error>, required PartialFunction[(Int, String), B]
// expect: 11:27: error: type mismatch: found (String, String) => String, required PartialFunction[(Int, String), B]
// expect: 12:23: error: type mismatch: found (String, String) => String, required ((Int, String)) => B
// A lambda of a tuple's elements needs as many parameters as the tuple has elements, and a
// declared parameter type has to take its element (scalac's `ptIsCorrectProduct`); otherwise
// it is a function of several parameters.
object Main:
  def main(args: Array[String]): Unit =
    val pairs = List((1, "a"), (2, "b"))
    println(pairs.collect((k, v, w) => v * k))
    println(pairs.collect((k: String, v: String) => v + k))
    println(pairs.map((k: String, v: String) => v + k))
