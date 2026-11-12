// jars: scala-library tapir-core-jvm magnolia123-jvm sttp-model-core-jvm sttp-shared-core-jvm sttp-shared-ws-jvm
// std: scala-library
//> using dep com.softwaremill.sttp.tapir::tapir-core:1.13.29
// Two of tapir's form codecs made in one method, the second for a class of two fields: each expansion's
// new methods are its own.
import sttp.tapir.*
import sttp.tapir.generic.Configuration

final case class Request(keyOne: String) derives Schema
final case class Other(wordTwo: String, age: Int) derives Schema

@main def run(): Unit =
  val c: Codec[String, Request, CodecFormat.XWwwFormUrlencoded] =
    Codec.formCaseClassCodec(using Configuration.default.withSnakeCaseMemberNames)
  val d: Codec[String, Other, CodecFormat.XWwwFormUrlencoded] =
    Codec.formCaseClassCodec(using Configuration.default.withSnakeCaseMemberNames)
  println(c.encode(Request("abc")))
  println(c.decode("key_one=xyz"))
  println(d.encode(Other("a", 3)))
  println(d.decode("word_two=b&age=4"))
