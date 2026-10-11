// A `{ case ... }` literal is dotty's `makeCaseLambda`: its parameter is the synthetic `x$1` (with
// several, `x$1, .., x$n`, the match on their tuple), which a name of the program written so refers
// to, and no other name of the program does.
@main def run(): Unit =
  val `x$1` = 99
  val f: Int => Int = { case _ => `x$1` }
  println(f(1))
  val `x$case` = 98
  val g: Int => Int = { case _ => `x$case` }
  println(g(1))
  val k: (Int, Int) => Int = { case (a, b) => a + b + `x$1` }
  println(k(1, 2))
  val pf: PartialFunction[Int, Int] = { case n if n > 0 => n + `x$case` }
  println(pf.isDefinedAt(0))
  println(pf(2))
  val m: (Int, String) => String = { case (a, s) => s * a }
  println(m(2, "ab"))
  println(List((1, 2), (3, 4)).map { case (a, b) => a * b })
