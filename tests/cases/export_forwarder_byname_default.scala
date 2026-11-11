class Ops(val s: String):
  def f(a: => Int = { println("fa"); 1 }, b: Int = { println("fb"); 2 }): Int = a * 10 + b + s.length * 100
object O:
  val ops = Ops("x")
  export ops.f
@main def run(): Unit =
  println(O.f(b = 5))
  println(O.f(3))
