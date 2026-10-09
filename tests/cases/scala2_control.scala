object Ctl {
  def classify(n: Int): String = {
    if (n < 0) "neg"
    else if (n == 0) "zero"
    else "pos"
  }
  def abs(n: Int): Int = if (n < 0) -n else n
  def sign(n: Int) = if (n < 0) -1 else if (n > 0) 1 else 0

  def loop(): Int = {
    var i = 0
    var acc = 0
    while (i < 5) {
      acc += i
      i += 1
    }
    while (i < 10)
      i += 1
    while (i < 12) i += 1
    acc + i
  }

  def fors(xs: List[Int], ys: List[Int]): List[Int] = {
    val a = for (x <- xs; y <- ys) yield x * y
    val b = for (x <- xs if x % 2 == 0; y <- ys; if y > 1) yield x + y
    val c = for {
      x <- xs
      y <- ys
      if x < y
    } yield (x, y)
    val d = for {
      x <- xs
      z = x * 2
    } yield z
    for (x <- xs) println(x)
    for (x <- xs)
      println(x + 100)
    for {
      x <- xs
    } println(x + 200)
    for ((p, q) <- c) println(p + q)
    for (x <- xs; y <- ys)
      if (x == y) println(s"same $x")
    a ++ b ++ d
  }

  def mixed(c: Boolean, d: Boolean): String = {
    val s1 = if (c) && d then "both" else "not both"
    val s2 = if (c) "c" else "not c"
    val s3 = if (c) {
      "braces"
    } else {
      "other"
    }
    val s4 =
      if (c)
        "indented"
      else
        "indented-else"
    val s5 = if (c) "one"; else "two"
    val s6 = if (c) (d) else (!d)
    val s7 = if (!c) "nc"
    else "c2"
    val s8 = if c then "new" else "old"
    val s9 = if (c) then "paren-then" else "paren-else"
    val s10 = if (c) || d then "or" else "nor"
    val s11 = if (c).equals(true) then "eq" else "ne"
    val s12 = if ((c)) "double" else "parens"
    var v = 0
    if (c) v = 1 else v = 2
    if (d)
      v += 10
    val s13 = if (v > 5) s"v=$v" else "small"
    val t = (1, 2)
    val s14 = if (t._1 < t._2) "lt" else "ge"
    if (c) println("side") else ()
    List(s1, s2, s3, s4, s5, s6, s7, s8, s9, s10, s11, s12, s13, s14).mkString(",")
  }

  def guards(xs: List[Int]): List[String] =
    xs.map { x =>
      x match {
        case n if (n < 0) => "neg"
        case n if n == 0 => "zero"
        case _ => "pos"
      }
    }

  def main(args: Array[String]): Unit = {
    println(classify(-1) + classify(0) + classify(1))
    println(abs(-3) + sign(-2) + sign(2))
    println(loop())
    println(fors(List(1, 2, 3), List(2, 3)))
    println(mixed(true, false))
    println(mixed(false, true))
    println(guards(List(-1, 0, 1)))
    var k = 0
    while (k < 3) { k += 1 }
    println(k)
    if (k == 3)
      println("three")
    if (k == 4) println("four")
    else println("not four")
  }
}
