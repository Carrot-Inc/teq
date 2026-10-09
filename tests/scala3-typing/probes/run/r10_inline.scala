object T:
  inline def twice(x: Int): Int = x * 2
  inline val Max = 10
  transparent inline def id(x: Int): Int = x
  inline def choose(inline b: Boolean): String = inline if b then "yes" else "no"
@main def run(): Unit =
  println(T.twice(3))
  println(T.Max)
  println(T.id(4))
  println(T.choose(true))
