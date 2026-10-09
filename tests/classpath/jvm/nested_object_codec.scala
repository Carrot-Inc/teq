// jars: scala-library zio-json-jvm magnolia-jvm zio-jvm zio-streams-jvm zio-stacktracer-jvm izumi-reflect-jvm izumi-reflect-boopickle-jvm scala-collection-compat-jvm
// std: scala-library
//> using dep dev.zio::zio-json:0.9.2
// A codec derived inside an object nested in a class, for a case class nested there too, in
// link mode: the mirror the derivation summons is made where it is summoned and holds the
// enclosing instance (its `fromProduct` constructs the class with it), the lambdas it makes
// stay with that instance, and magnolia's `inline$returningNone` accessor is called as the
// jar declares it.
import zio.json.*

enum Kind:
  case A, B

class Outer(val prefix: String):
  final case class Response(text: String, n: Int)
  object Response:
    implicit val codec: JsonCodec[Response] = DeriveJsonCodec.gen
    val names: List[String] = Kind.values.toList.map(k => prefix + k.toString)
  def encoded: String = Response("t", 1).toJson + Response.names.mkString(",")
  def decoded: Either[String, Response] = """{"text":"u","n":2}""".fromJson[Response]

final case class Top(text: String, n: Int)
object Top:
  implicit val codec: JsonCodec[Top] = DeriveJsonCodec.gen

@main def run(): Unit =
  val o = Outer("#")
  println(o.encoded)
  println(o.decoded)
  println(Top("v", 3).toJson + """{"text":"w","n":4}""".fromJson[Top])
