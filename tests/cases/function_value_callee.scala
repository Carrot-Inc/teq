object Main:
  def render(n: Int, s: String): String = s + n
  def identity[A](x: A): A = x
  def join(xs: String*): String = xs.mkString("/")

  def main(args: Array[String]): Unit =
    val reordered = (s, n) => render(n, s)
    val instantiated = n => identity[Int](n)
    val spread = xs => join(xs*)
    val bounded: List[Int => Int] = List(x => x + 1)
    val paired: ((Int, String)) => String = (n, s) => render(n, s)
    val patterns: (Int, String) => String = { case (n, s) => render(n, s) }
    val guarded: (Int, String) => String = {
      case (n, s) if n > 0 => render(n, s)
      case (_, s) => s
    }
    val contextual: Int ?=> String = summon[Int].toString
    println(reordered("n=", 3))
    println(instantiated(7))
    println(spread(Seq("a", "b")))
    println(bounded.head(8))
    println(paired((4, "p=")))
    println(patterns(5, "m="))
    println(guarded(2, "g="))
    println(guarded(0, "zero"))
    println(contextual(using 6))
