// `Option` is an `Iterable` through the implicit conversion `Option.option2Iterable` of its
// companion, as in scala-library: `++` on an option, an option where an iterable is expected.
object Main:
  def total(xs: Iterable[Int]): Int = xs.sum
  def main(args: Array[String]): Unit =
    val warning: Option[Int] = Some(1)
    val neutral: Option[String] = None
    val both = warning.map(_.toString) ++ neutral.map(_ + "!")
    println(both)
    println(warning.map(_.toString) ++ Some("x").map(_ + "!"))
    val ys: Iterable[Int] = warning
    println(ys.toList)
    println(total(Some(41)) + total(None))
    println(List(1, 2) ++ Some(3) ++ None)
    println((Some(2) ++ List(3, 4)).toList)
    println(Some(5).toList ++ Option(6))
    val words = Some("a") ++ Some("b") ++ None
    println(words.mkString("-"))
