// expect: 13:27: error: type mismatch: found (<error>, <error>, <error>) => <error>, required PartialFunction[(Int, String), Any]
// expect: 14:27: error: type mismatch: found (String, String) => String, required PartialFunction[(Int, String), Any]
// expect: 15:23: error: type mismatch: found (String, String) => String, required ((Int, String)) => Any
// A lambda of a tuple's elements needs as many parameters as the tuple has elements, and a
// declared parameter type has to take its element (scalac's `ptIsCorrectProduct`); otherwise
// it is a function of several parameters. The required type shows `collect`'s and `map`'s `B` as
// the call's typing leaves it, before the call solves it (dotty's attempt's constraint): open,
// so its upper bound `Any`, where scalac's closure has instantiated it to `String`; with an
// error found, `Any` too (scalac: a syntax error, E086).
object Main:
  def main(args: Array[String]): Unit =
    val pairs = List((1, "a"), (2, "b"))
    println(pairs.collect((k, v, w) => v * k))
    println(pairs.collect((k: String, v: String) => v + k))
    println(pairs.map((k: String, v: String) => v + k))
