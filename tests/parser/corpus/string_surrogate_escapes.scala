// A surrogate pair written as two unicode escapes is one supplementary character: a string of
// two UTF-16 units.
@main def run(): Unit =
  val s = "a😀b"
  println(s.length + " " + s.codePointAt(1) + " " + s.charAt(1).toInt + " " + s.charAt(2).toInt)
  println(s)
  println("𝄞".codePointCount(0, 2) + " " + "xA\uuD83D\uDE00".length)
