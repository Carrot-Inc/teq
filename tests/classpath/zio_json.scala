// jars: scala-library zio-json
// A Scala.js jar (`_sjs1_3`) checks against scala-library alone: zio-json's encoders and
// decoders resolve by summon, and `decodeJson(str: CharSequence)` takes a String.
import zio.json.*
object Main:
  def main(args: Array[String]): Unit =
    println(JsonEncoder[Int].encodeJson(1, None))
    println(JsonDecoder[List[Int]].decodeJson("[1, 2, 3]"))
    println(JsonEncoder[Map[String, Boolean]].encodeJson(Map("a" -> true), None))
