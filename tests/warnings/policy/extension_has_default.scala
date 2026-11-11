class Ops(val s: String):
  inline def pad(inline n: Int = 1, inline c: Int = 20): Int = n + c + s.length
  def wrap(n: Int = 3): String = s * n
extension (s: String)
  def ops: Ops = Ops(s)
  export ops.{pad, wrap}
object Main:
  extension (s: String = "x") def tag: String = s + "!"
  extension (s: String) def twice(n: Int = 2): String = s * n
  extension [T](using u: DummyImplicit)(t: T = 0) def show: String = t.toString
  def main(args: Array[String]): Unit =
    println("a".tag)
    println("b".twice())
    println("ab".pad(c = 5))
