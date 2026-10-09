import scala.util.control.NonFatal
import scala.util.{Failure, Success, Try}

class ParseError(message: String, val line: Int) extends Exception(message)
final class TooLong(val limit: Int) extends RuntimeException("longer than " + limit)
case class Rejected(reason: String) extends Exception(reason)
class Wrapped(cause: Throwable) extends RuntimeException("wrapped", cause)

def parseAge(text: String, line: Int): Int =
  if text.isEmpty then throw new ParseError("empty age", line)
  val n = text.toInt
  if n < 0 then throw Rejected("negative")
  n

def describe(text: String, line: Int): String =
  try s"age ${parseAge(text, line)}"
  catch
    case e: ParseError => s"parse error at ${e.line}: ${e.getMessage}"
    case Rejected(reason) => "rejected: " + reason
    case e: NumberFormatException => "not a number: " + e.getMessage

def limited(text: String): String =
  try
    if text.length > 3 then throw new TooLong(3)
    text.toUpperCase
  catch
    case e: TooLong if e.limit == 3 => "limit " + e.limit
  finally
    println("checked " + text)

def nested(text: String): String =
  try
    try parseAge(text, 0).toString
    catch
      case e: ParseError => throw new Wrapped(e)
  catch
    case w: Wrapped => "outer saw " + w.getCause.getMessage

def rethrown(text: String): Int =
  try parseAge(text, 1)
  catch
    case e: ParseError =>
      println("logging " + e.getMessage)
      throw e

def sum(items: List[String]): Int =
  try items.map(s => parseAge(s, 0)).sum
  catch case NonFatal(e) => -1

val handler: PartialFunction[Throwable, String] =
  case e: ParseError => "pf: " + e.getMessage

def viaHandler(text: String): String =
  try parseAge(text, 2).toString
  catch handler

@main def main(): Unit =
  println(describe("42", 1))
  println(describe("", 2))
  println(describe("-1", 3))
  println(describe("x1", 4))
  println(limited("ab"))
  println(limited("abcd"))
  println(nested("7"))
  println(nested(""))
  println(try rethrown("") catch { case e: ParseError => "caught again: " + e.getMessage })
  println(sum(List("1", "2", "3")))
  println(sum(List("1", "", "3")))
  println(viaHandler("5"))
  println(viaHandler(""))
  println(try viaHandler("no") catch { case e: NumberFormatException => "fell through" })
  val results = List("10", "", "20").map(s => Try(parseAge(s, 0)))
  println(results.map {
    case Success(v) => "ok " + v
    case Failure(e: ParseError) => "failed " + e.getMessage
    case Failure(e) => "other " + e.getMessage
  })
  val messages = List("1", "-1", "z").map(s => try parseAge(s, 0).toString catch { case e: Exception => e.getMessage })
  println(messages)
  println(new ParseError("m", 1).toString)
  println(Rejected("r").toString)
  println(new TooLong(2).getMessage)
  println(new Wrapped(Rejected("inner")).getCause)
  println(new RuntimeException().getMessage == null)
  println(new IllegalArgumentException("bad").toString)
  var closed = 0
  def compute(fail: Boolean): Int =
    try
      if fail then throw new IllegalStateException("failing")
      1
    finally
      closed += 1
  println(compute(false))
  println(try compute(true) catch { case e: IllegalStateException => 2 })
  println(closed)
  val value: Int = try 10 finally println("finally runs")
  println(value)
