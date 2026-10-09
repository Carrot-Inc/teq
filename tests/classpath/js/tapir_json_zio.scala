// jars: scala-library tapir-core tapir-json-zio zio-json magnolia123 zio sttp-model sttp-shared-core sttp-shared-ws scala-java-time
//> using dep com.softwaremill.sttp.tapir::tapir-core:1.13.29
//> using dep com.softwaremill.sttp.tapir::tapir-json-zio:1.13.29
//> using dep dev.zio::zio-json:0.9.2
// tapir 1.13.29's zio-json integration from its jar: `jsonBody[A]` over zio-json 0.9.2 codecs
// derived by Magnolia, `Codec.json`, encoding and decoding through the codec, decode failures
// as `DecodeResult.Error` with `JsonDecodeException`, and the schema the body carries.
package tapirjsonzio

import sttp.tapir.*
import sttp.tapir.json.zio.*
import sttp.tapir.generic.auto.*
import zio.json.*

case class Address(street: String, zip: Option[String]) derives JsonCodec, Schema
case class Pet(id: Int, name: String, tags: List[String], address: Address, score: BigDecimal) derives JsonCodec, Schema

enum Kind derives JsonCodec, Schema:
  case Dog, Cat

case class Wrapper(kind: Kind, count: Long) derives JsonCodec, Schema

object Main:
  def main(args: Array[String]): Unit =
    val body = jsonBody[Pet]
    println(body.show)
    println(body.codec.format.mediaType.toString + " " + body.bodyType)
    val pet = Pet(1, "rex", List("a", "b"), Address("main", None), BigDecimal("2.50"))
    val encoded = body.codec.encode(pet)
    println(encoded)
    println(body.codec.decode(encoded).toString)
    println(body.codec.decode("""{"id": "x"}""").toString)
    println(body.codec.decode("not json").getClass.getSimpleName)
    body.codec.decode("""{"id": 1, "name": "n", "tags": [], "address": {"street": 5}, "score": 1}""") match
      case DecodeResult.Error(original, e: DecodeResult.Error.JsonDecodeException) =>
        println(original.length.toString + " " + e.errors.map(err => err.msg + "@" + err.path.map(_.name)).mkString(";") + " " + e.underlying.getMessage)
      case other => println("unexpected " + other)
    println(body.codec.schema.name.toString + " " + body.codec.schema.schemaType.getClass.getSimpleName)
    val wrapped = jsonBody[Wrapper]
    println(wrapped.codec.encode(Wrapper(Kind.Cat, 7L)) + " " + wrapped.codec.decode("""{"kind":"Dog","count":3}"""))
    println(wrapped.codec.decode("""{"kind":"Bird","count":3}""").toString)
    val opt = jsonBody[Option[Pet]]
    println(opt.codec.decode("").toString + " " + opt.codec.encode(None) + " " + opt.codec.decode("null"))
    val list = jsonBody[List[Address]]
    println(list.codec.encode(List(Address("a", Some("1")), Address("b", None))) + " " + list.codec.decode("[]"))
    val plain = Codec.json[Int](s => DecodeResult.Value(s.trim.toInt))(_.toString)
    println(plain.decode(" 42 ").toString + " " + plain.encode(7) + " " + plain.format.mediaType)
    val ep = endpoint.post.in("pets").in(jsonBody[Pet]).out(jsonBody[Address]).errorOut(jsonBody[Wrapper])
    println(ep.show)
    println(ep.showDetail)
    val codec = summon[JsonCodec[Pet]]
    println(codec.decodeJson(encoded).map(_.name).toString + " " + pet.toJson)
