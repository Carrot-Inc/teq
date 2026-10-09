// Scala 2 control syntax next to Scala 3 forms, and the shapes dotc's condExpr tells apart.
object Shapes:
  def a(c: Boolean, d: Boolean): Int =
    if (c) && d then 1 else 2

  def b(c: Boolean, d: Boolean): Int =
    if (c) || d then 1
    else 2

  def c(n: Int): Int =
    if (n) == 1 then 10 else 20

  def d(f: Int => Boolean): String =
    if (f)(3) then "applied" else "not"

  def e(xs: List[Int]): Int =
    if (xs).isEmpty then 0 else xs.head

  def g(n: Int): Int = if (n > 0) (n) else (-n)

  def h(o: Option[Int]): Int =
    if (o) match
      case Some(v) => v > 0
      case None => false
    then 1
    else 0

  def i(c: Boolean): Int =
    if (c)
      val t = 1
      t + 1
    else
      val u = 5
      u * 2

  def j(c: Boolean): String =
    if (c)
    {
      "brace on next line"
    }
    else
    {
      "else brace on next line"
    }

  def k(n: Int): Int =
    var i = 0
    var s = 0
    while (i < n)
    {
      s += i
      i += 1
    }
    while (i < n + 2) do i += 1
    while (i < n + 4)
      i += 1
    s + i

  def l(xs: List[Int], ys: List[Int]): List[Int] =
    for (x <- xs; y <- ys if x != y) yield x - y

  def m(xs: List[Int]): List[Int] =
    for {
      x <- xs
      if x > 1
      y = x * 10
    }
    yield y

  def n(xs: List[List[Int]]): List[Int] =
    for (x <- xs; y <- x)
    yield y

  def o(xs: List[Int]): Unit =
    for (x <- xs)
    {
      println(x)
    }
    for (x <- xs) do println(x + 1)
    for x <- xs do println(x + 2)
    for (x <- xs) if (x > 1) println(s"big $x") else println(s"small $x")

  def p(pairs: List[(Int, Int)]): Int =
    var s = 0
    for ((a, b) <- pairs) s += a * b
    for (a, b) <- pairs do s += a + b
    s

  def q(c: Boolean): Int =
    if (c) 1; else 2

  def r(c: Boolean, d: Boolean): String =
    if (c) if (d) "cd" else "c" else "none"

  def s(n: Int): String =
    if (n < 0) "neg" else if (n == 0) "zero" else if (n < 10) "small" else "big"

  def t(xs: List[Int]): Int =
    var total = 0
    for (x <- xs)
      if (x % 2 == 0)
        total += x
      else
        total -= x
    total

@main def main(): Unit =
  import Shapes.*
  println(List(a(true, true), a(true, false), b(false, true), b(false, false), c(1), c(2)))
  println(d(_ > 2) + d(_ > 5) + e(Nil) + e(List(7)) + g(-4) + g(4))
  println(List(h(Some(1)), h(Some(-1)), h(None), i(true), i(false)))
  println(j(true) + " / " + j(false))
  println(k(3))
  println(l(List(1, 2), List(2, 3)))
  println(m(List(1, 2, 3)))
  println(n(List(List(1), List(2, 3))))
  o(List(1, 2))
  println(p(List((1, 2), (3, 4))))
  println(q(true) + q(false))
  println(r(true, true) + r(true, false) + r(false, true))
  println(s(-1) + s(0) + s(5) + s(50))
  println(t(List(1, 2, 3, 4)))
