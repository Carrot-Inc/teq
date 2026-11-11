class Ops(val s: String):
  def f(a: Int, b: Int = 2): Int = a * 10 + b + s.length * 100
  def g(a: Int)(b: Int = 3): Int = a * 10 + b + s.length * 100
extension (s: String)
  def ops: Ops = Ops(s)
  export ops.{f, g}
@main def run(): Unit =
  println("x".f(b = 5))
