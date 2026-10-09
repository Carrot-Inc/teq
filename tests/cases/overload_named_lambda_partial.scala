// A function literal meets a partial function: as a positional argument of an overloaded method,
// or named where no alternative is to be chosen among (one method, one alternative left by the
// number or the shape of the arguments, the choice made by an earlier list) or where it is a
// `{ case ... }` literal, and a function parameter is chosen over a partial function one.
object O:
  def f(pf: PartialFunction[Int, Int]): Int = pf(1)
  def f(pf: PartialFunction[Int, Int], x: Int): Int = pf(x)
  def s(pf: PartialFunction[Int, Int]): Int = pf(1)
  def s(pf: Int): Int = pf
  def c(n: Int)(pf: PartialFunction[Int, Int]): Int = pf(n)
  def c(t: String)(pf: PartialFunction[Int, Int]): Int = pf(t.length)
  def k(f: Int => Int): String = "function " + f(1)
  def k(f: PartialFunction[Int, Int]): String = "partial " + f(1)
@main def run(): Unit =
  println(Map(1 -> 2).collect(kv => kv._1 + kv._2).mkString(","))
  println(Map(1 -> 2).collect(pf = { case (k, v) => k + v }).mkString(","))
  println(List(1, 2).collect(pf = n => n + 1))
  println(O.f(pf = x => x + 1))
  println(O.s(pf = x => x + 1))
  println(O.c(1)(pf = x => x + 1))
  println(O.k(f = x => x + 1))
