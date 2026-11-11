object Main:
  def x(s: String, i: Int): Int = s.length + i
  def f(a: Int, b: Int): Int = a + b
  def g(a: Int)(b: Int = 4, c: Int = 5): Int = a + b + c
  class C(val a: Int, val b: String)
  case class P(a: Int, b: String = "s")
  def o(a: Int, b: Int): Int = a + b
  def o(a: String, b: Int): Int = b
  def h(a: Int, k: Int => Int): Int = k(a)
  def h(a: String, k: Int => Int): Int = k(1)
  inline def here: String = Show.args(f(1, b = x("here", 2)))
  inline def pass(inline e: Any): String = Show.args(e)
  def main(args: Array[String]): Unit =
    println(Show.args(f(1, b = x("b", 2))))
    println(Show.args(f(a   =   1, b = (x("b", 2)))))
    println(Show.args(g(1)(c = 3)))
    println(Show.args(new C(a = 1, b = "two")))
    println(Show.args(f(1,
      b =
        x("b", 2))))
    println(Show.args(f(b = x("b", 2), a = 1)))
    println(Show.args(P(a = 1).copy(b = "z")))
    println(Show.args(o(1, b = x("b", 2))))
    println(Show.args(o("s", b = 2)))
    println(Show.args(h(1, k = _ + 1)))
    println(Show.args(h(1, k = (i: Int) => i)))
    println(here)
    println(pass(f(1, b = x("pass", 2))))
    println(Wrap.there)
