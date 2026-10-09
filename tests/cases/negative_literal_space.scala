// A minus before a number token is the number's sign whatever lies between them on the line (dotc's
// `Parsers.prefixExpr` and `simplePattern`): `- 1.5` is the literal `-1.5`, a Float where one is
// expected and read from its digits as one; `- 2147483648` is an Int; a selection after it applies
// to the negative number, a comment on the line between them too. A suffixed zero is no number too
// small. A line break inside a block comment ends a statement as any line break does.
@main def run(): Unit =
  val a: Float = - 1.5
  val b: Float = - 1.1
  val c: Float = -  1.1
  println(a == -1.5f)
  println(b == -1.1f)
  println(c == -1.1f)
  val i = - 2147483648
  println(i)
  val l = - 9223372036854775808L
  println(l)
  println(- 1.toString)
  println(- 2.abs)
  def three: Int = -3
  println(three match { case - 3 => "minus three"; case _ => "other" })
  val x = 3
  println(x - 1)
  println(x -1)
  println(0e-999f == 0f)
  println(0.0e999d == 0d)
  val g: Float = - /* gap */ 1.1
  println(g == -1.1f)
  val p = 1 /*
  */ val q = 2
  println(p + q)
