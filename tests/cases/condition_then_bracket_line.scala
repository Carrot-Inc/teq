// After a condition in parentheses, a line further in that begins with a bracket goes on with the condition's
// line: it is the branch or the body itself and opens no block of its own, for an `if`, a `while` and a `for`;
// after a blank line the bracket begins the branch's block. scalac prints (3,7,6,List(2, 3),5).
object Test:
  def a(c: Boolean): Int =
    if (c)
      (1 + 2)
    else
      (3 + 4)
  def b(n: Int): Int =
    var i = 0
    var s = 0
    while (i < n)
      { s += i; i += 1 }
    s
  def d(xs: List[Int]): List[Int] =
    for (x <- xs)
      yield x + 1
  def e(c: Boolean): Int =
    if (c)

      (5)
    else 6
  def main(args: Array[String]): Unit =
    println((a(true), a(false), b(4), d(List(1, 2)), e(true)))
