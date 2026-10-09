extension (x: String)
  def baz(y: String): String =
    val x = y
    x + "!"

class C(val n: Int):
  def m(k: Int): Int =
    val n = k + 1
    n * 2

object Loop:
  def count(n: Int, acc: List[Int]): List[Int] =
    if n == 0 then acc
    else
      val n2 = n - 1
      val acc2 = n :: acc
      count(n2, acc2)
  def shadowed(n: Int, acc: Int): Int =
    if n == 0 then acc
    else
      val n = acc + 1
      shadowed(0, n)
  def f(x: Int): Int =
    val g = (x: Int) => x * 2
    val (a, b) = (x, g(x))
    val h = (y: Int) =>
      val x = y + 1
      x + a
    h(b)

@main def main(): Unit =
  println("abc".baz("def"))
  println(C(1).m(2))
  val v = 1
  val w =
    val v = 2
    v + 1
  println(v + " " + w)
  println(Loop.count(3, Nil))
  println(Loop.shadowed(3, 0))
  println(Loop.f(3))
