// A value that JavaScript throws is a js.JavaScriptException to `catch`, and throwing that
// exception throws the value itself again, as under Scala.js.
import scala.util.control.NonFatal

def parseJson(text: String): Any = js.call(js.global("JSON"), "parse", text)

def describe(body: => Any): String =
  try "ok " + body.toString
  catch
    case js.JavaScriptException(e) => "js error: " + js.typeOf(e) + " " + e.toString.takeWhile(_ != ':')
    case e: Throwable => "throwable: " + e.getMessage

def messageOf(body: => Any): String =
  try body.toString
  catch case NonFatal(e) => e.toString + " / " + e.getMessage

def onlySpecific(body: => Any): String =
  try body.toString
  catch case e: NumberFormatException => "number"

@main def main(): Unit =
  println(describe(parseJson("[1, 2]")))
  println(describe(parseJson("{oops")))
  println(describe(js.throwError("a string")))
  println(describe(js.throwError(42)))
  println(describe(throw new IllegalStateException("scala side")))
  println(messageOf(parseJson("{oops")).takeWhile(_ != ':'))
  println(messageOf(js.throwError("plain")))
  println(messageOf("7".toInt / 0))
  println(try onlySpecific(js.throwError("passes through")) catch { case js.JavaScriptException(e) => "outer got " + e.toString })
  println(try onlySpecific("x".toInt) catch { case js.JavaScriptException(e) => "outer got " + e.toString })
  val rethrown = js.tryCatch(() => { try js.throwError("raw") catch { case e: js.JavaScriptException => throw e } })(v => "raw again: " + js.typeOf(v) + " " + v.toString)
  println(rethrown)
  val wrappedTwice = try { try js.throwError("once") catch { case e: Throwable => throw new RuntimeException("wrapped " + e.getMessage) } } catch { case e: RuntimeException => e.getMessage }
  println(wrappedTwice)
  println(js.tryFinally(() => 1)(() => println("finalizer")))
