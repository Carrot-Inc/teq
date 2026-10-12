// A name of an overloaded method with a parameterless alternative, passed to an overloaded method, is
// typed ahead as that alternative, which dotty's isAsSpecific prefers to every method: its type picks the
// callee's alternative (sttp's `request.response(asStringAlways)`).
class As[T](val name: String)
class StreamAs[T, S](val name: String)

object Responses:
  def asText: As[String] = As("text")
  def asText(charset: String): As[String] = As("text " + charset)

class Request:
  def response[T](ra: As[T]): String = "plain " + ra.name
  def response[T, S](ra: StreamAs[T, S]): String = "stream " + ra.name

@main def run(): Unit =
  import Responses.*
  println(Request().response(asText))
  println(Request().response(asText("utf-8")))
