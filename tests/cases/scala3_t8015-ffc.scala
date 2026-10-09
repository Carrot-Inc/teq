// Adapted from scala3 tests/run/t8015-ffc.scala (Apache-2.0, see tests/scala3/README.md); replaced: extends App.
object Test {
  def main(args: Array[String]): Unit = ()
  val ms = s"""This is a long multiline interpolation
  with \u000d\u000a CRLF embedded."""
  assert(ms.linesIterator.size == 3, s"lines.size ${ms.linesIterator.size}")
  assert(ms contains "\r\n CRLF", "no CRLF")
}
