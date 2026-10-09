// An outlined function's parameter wrapped by a same-number widening (`ByteToInt` of an inline
// `Byte` denominator) stays a parameter to the JavaScript emitter's literal-based rules: the
// division keeps its zero check at every expansion, where the representative's literal would
// have dropped it; and the widenings are no part of the outline's shape, so the shared function
// is the one master named.
object M:
  inline def divided(inline denominator: Byte, n: Int): Int =
    val a = n + 1
    val b = n + 2
    val c = n + 3
    val d = n + 4
    (a + b + c + d) / denominator
  inline def remainder(inline denominator: Short, n: Int): Int =
    val a = n + 1
    val b = n + 2
    val c = n + 3
    val d = n + 4
    (a + b + c + d) % denominator
  inline def score(x: Int): Double =
    val a = x + 1
    val b = x + 2
    val c = x + 3
    val d = x + 4
    (a.toDouble + b.toDouble + c.toDouble + d.toDouble) / 3 + 0.25

@main def run(): Unit =
  val n = "ab".length
  println(M.divided(2, n))
  try println(M.divided(0, n))
  catch case _: ArithmeticException => println("division by zero")
  println(M.remainder(3, n))
  try println(M.remainder(0, n))
  catch case _: ArithmeticException => println("remainder by zero")
  println(M.score(n + 1))
  println(M.score(n + 2))
