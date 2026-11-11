class Ops(val s: String):
  def f(a: Int)(b: Int = 2, c: Int = 3): Int = a * 100 + b * 10 + c + s.length * 1000
  inline def g(a: Int)(inline b: Int = 4, inline c: Int = 5): Int = a * 100 + b * 10 + c + s.length * 1000
extension (s: String)
  def ops: Ops = Ops(s)
  export ops.{f, g}
@main def run(): Unit =
  println("x".f(1)(c = 9))
  println("xy".f(2)(7, 8))
  println("x".g(1)(c = 9))
  println("xy".g(2)(7, 8))
