// scala-library's `patch` of a string, which gives a string, and `linesWithSeparators`, the lines
// with the break that ends each (zio-test colours the lines of a report with the one, utest
// splits its diffs with the other).
@main def run(): Unit =
  println("hello world".patch(6, "there", 5))
  println("abc".patch(1, "XY", 0))
  println("abc".patch(-2, "<", 1))
  println("abc".patch(10, ">", 3))
  println("abcdef".patch(2, "", 2))
  val s: String = "x\n".patch(2, "\u001b[32m", 0)
  println(s.length)
  println("one\ntwo\r\nthree\rfour".linesWithSeparators.map(_.replace("\r", "\\r").replace("\n", "\\n")).toList)
  println("end\n".linesWithSeparators.toList.size)
  println("".linesWithSeparators.hasNext)
