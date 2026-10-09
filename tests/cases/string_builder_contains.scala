// `contains` of a builder looks for an element, a `Char`.
@main def run(): Unit =
  val sb = new StringBuilder("abcabc")
  println(sb.contains('a'))
  println(sb.contains('z'))
  val element: Any = 'c'
  println(sb.contains(element))
  println(new StringBuilder().contains('a'))
  println(sb.contains("ab"))
