//> using scala 3.8.4
@main def run(): Unit =
  val half: PartialFunction[Int, Int] = { case n if n % 2 == 0 => n / 2 }
  val small: PartialFunction[Int, String] = { case n if n < 10 => "small " + n }
  val chained = half.andThen(small)
  println(chained.isDefinedAt(8))
  println(chained.isDefinedAt(80))
  println(chained.isDefinedAt(7))
  val literal = half.andThen { case 2 => "two" }
  println(literal.isDefinedAt(4))
  println(literal.isDefinedAt(8))
  println(literal.lift(4))
  val total = half.andThen(n => n + 1)
  println(total.isDefinedAt(8))
  println(total.isDefinedAt(7))
  println(total(8))
  val pre = half.compose[String] { case s if s.nonEmpty => s.length }
  println(pre.isDefinedAt("abcd"))
  println(pre.isDefinedAt(""))
  println(pre.isDefinedAt("abc"))
  val plain: String => Int = half.compose((s: String) => s.length)
  println(plain("abcdef"))
  println(List(2, 3, 4).collect(half.andThen(small)))
