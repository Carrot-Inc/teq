//> using scala 3.8.4
object Formats:
  def fmt(n: Int): String = "#" + n
  def fmt(s: String): String = "'" + s + "'"
  def fmt(n: Int, width: Int): String = ("#" + n).padTo(width, '.')
  def plain(n: Int): String = n.toString

trait Scanner:
  def scan(page: Int): String = "page " + page
  def scan(from: Int, to: Int): String = "pages " + from + "-" + to

object Flatbed extends Scanner:
  def scan(name: String): String = "scan " + name

object Office:
  export Formats.*
  export Flatbed.scan as digitise

object Desk:
  export Formats.{fmt as format, plain}
  export Flatbed.*

@main def run(): Unit =
  println(Office.fmt(1))
  println(Office.fmt("x"))
  println(Office.fmt(1, 4))
  println(Office.plain(3))
  println(Office.digitise(3))
  println(Office.digitise(1, 2))
  println(Office.digitise("doc"))
  println(Desk.format(7))
  println(Desk.format("y"))
  println(Desk.scan(4))
  println(Desk.scan("z"))
  import Office.*
  println(fmt(2))
  println(digitise(9, 10))
  val f: String => String = Desk.format
  println(f("w"))
  println(List(1, 2).map(Office.fmt))
