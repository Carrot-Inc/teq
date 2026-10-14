// scala-library's CommandLineParser, which a typed @main method's proxy calls: each argument parsed by the
// FromString given of its type (a program's own among them), the rest of them as a list, and a ParseError
// that names the argument's index, shown as the proxy shows it.
import scala.util.CommandLineParser
import scala.util.CommandLineParser.{FromString, ParseError}

case class Point(x: Int, y: Int)
given FromString[Point] with
  def fromString(s: String): Point =
    s.split(",") match
      case Array(a, b) => Point(a.toInt, b.toInt)
      case _ => throw new IllegalArgumentException("no point: " + s)

object Main:
  def main(args: Array[String]): Unit =
    val line = Array("7", "true", "2.5", "1,2", "x", "y")
    println(CommandLineParser.parseArgument[Int](line, 0))
    println(CommandLineParser.parseArgument[Boolean](line, 1))
    println(CommandLineParser.parseArgument[Double](line, 2))
    println(CommandLineParser.parseArgument[Point](line, 3))
    println(CommandLineParser.parseRemainingArguments[String](line, 4))
    println(s"${summon[FromString[Long]].fromStringOption("12")} ${summon[FromString[Long]].fromStringOption("z")}")
    for n <- List(0, 1, 2, 3) do
      try CommandLineParser.parseArgument[Int](Array("a", "b", "c"), n)
      catch case e: ParseError => CommandLineParser.showError(e)
    try CommandLineParser.parseArgument[Point](line, 4)
    catch case e: ParseError => println(s"${e.idx} ${e.msg}")
