// Function literals whose bodies are blocks ending in a match, typed on the workers after the fork.
object A:
  def f(xs: List[Int]): List[String] = xs.map { x => val y = x + 1; y match { case 2 => "two"; case _ => "other" } }
object B:
  def g(xs: List[Int]): List[Int] = xs.map { x => val y = x * 2; y match { case 4 => 40; case n => n } }
object C:
  def h(xs: List[String]): List[Int] = xs.map { s => val n = s.length; n match { case 0 => -1; case k => k } }
@main def run(): Unit =
  println(A.f(List(1, 2)))
  println(B.g(List(2, 3)))
  println(C.h(List("", "ab")))
