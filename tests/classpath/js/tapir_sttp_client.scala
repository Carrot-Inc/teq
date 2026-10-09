// jars: scala-library tapir-core tapir-client tapir-sttp-client4 tapir-json-zio zio-json magnolia123 zio sttp-client4-core sttp-model sttp-shared-core sttp-shared-ws scala-java-time
//> using dep com.softwaremill.sttp.tapir::tapir-core:1.13.29
//> using dep com.softwaremill.sttp.tapir::tapir-sttp-client4:1.13.29
//> using dep com.softwaremill.sttp.tapir::tapir-json-zio:1.13.29
//> using dep dev.zio::zio-json:0.9.2
// tapir 1.13.29's sttp client4 interpreter from its jar: `SttpClientInterpreter().toRequest`
// and `toSecureRequest` turn endpoints with path captures, queries, headers, string and JSON
// bodies, `oneOf` error outputs and a bearer token into sttp requests, whose responses a
// synchronous `BackendStub` answers with canned bodies and codes, decoded through the
// endpoint's outputs into `DecodeResult[Either[E, O]]`.
package tapirsttpclient

import sttp.client4.*
import sttp.client4.testing.BackendStub
import sttp.model.{Header, StatusCode}
import sttp.tapir.*
import sttp.tapir.client.sttp4.SttpClientInterpreter
import sttp.tapir.json.zio.*
import sttp.tapir.generic.auto.*
import zio.json.*

case class Pet(id: Int, name: String, tags: List[String]) derives JsonCodec, Schema
case class NewPet(name: String, tags: List[String]) derives JsonCodec, Schema
sealed trait Failure
case class NotFound(what: String) extends Failure
case class Forbidden(reason: String) extends Failure

object Api:
  val getPet: PublicEndpoint[(Int, Option[String]), String, Pet, Any] =
    endpoint.get.in("pets" / path[Int]("id")).in(query[Option[String]]("expand")).out(jsonBody[Pet]).errorOut(stringBody)
  val createPet: PublicEndpoint[(String, NewPet), Failure, (Pet, String), Any] =
    endpoint.post
      .in("pets")
      .in(header[String]("X-Trace"))
      .in(jsonBody[NewPet])
      .out(statusCode(StatusCode.Created).and(jsonBody[Pet]).and(header[String]("Location")))
      .errorOut(
        oneOf[Failure](
          oneOfVariant(StatusCode.NotFound, plainBody[String].map(NotFound(_))(_.what)),
          oneOfVariant(StatusCode.Forbidden, plainBody[String].map(Forbidden(_))(_.reason))
        )
      )
  val whoAmI: Endpoint[String, Unit, Unit, String, Any] =
    endpoint.get.securityIn(auth.bearer[String]()).in("me").out(stringBody)
  val count: PublicEndpoint[(String, Int), Unit, Int, Any] =
    endpoint.get.in("count" / path[String]("kind")).in(query[Int]("min")).out(plainBody[Int])

object Main:
  def main(args: Array[String]): Unit =
    val base = Some(uri"https://api.example.com/v1")
    val interpreter = SttpClientInterpreter()

    val get = interpreter.toRequest(Api.getPet, base)((42, Some("owner")))
    println(get.show())
    println(get.method.toString + " " + get.uri.path + " " + get.uri.paramsMap)
    val getNone = interpreter.toRequest(Api.getPet, base)((7, None))
    println(getNone.uri.toString)

    val create = interpreter.toRequest(Api.createPet, base)(("t-1", NewPet("rex", List("dog", "big"))))
    println(create.show(includeBody = true, includeHeaders = true))

    val me = interpreter.toSecureRequest(Api.whoAmI, base)("secret")(())
    println(me.show() + " " + me.header("Authorization"))

    val backend = BackendStub.synchronous
      .whenRequestMatches(r => r.uri.path.endsWith(List("pets", "42")))
      .thenRespondAdjust("""{"id":42,"name":"rex","tags":["dog"]}""")
      .whenRequestMatches(r => r.uri.path.endsWith(List("pets", "7")))
      .thenRespondAdjust("no such pet", StatusCode.BadRequest)
      .whenRequestMatches(r => r.uri.path.endsWith(List("pets", "9")))
      .thenRespondAdjust("""{"id":9,"name":"bad"}""")
      .whenRequestMatches(r => r.method == sttp.model.Method.POST && r.header("X-Trace").contains("t-1"))
      .thenRespond(sttp.client4.testing.ResponseStub.adjust("""{"id":43,"name":"rex","tags":["dog","big"]}""", StatusCode.Created, List(Header("Location", "/v1/pets/43"))))
      .whenRequestMatches(r => r.method == sttp.model.Method.POST && r.header("X-Trace").contains("t-2"))
      .thenRespondAdjust("not yours", StatusCode.Forbidden)
      .whenRequestMatches(r => r.method == sttp.model.Method.POST)
      .thenRespondAdjust("unknown owner", StatusCode.NotFound)
      .whenRequestMatches(r => r.uri.path.startsWith(List("v1", "me")))
      .thenRespondAdjust("alice")
      .whenRequestMatches(r => r.uri.path.startsWith(List("v1", "count")))
      .thenRespondAdjust("12")

    println(get.send(backend).body)
    println(getNone.send(backend).body)
    println(interpreter.toRequest(Api.getPet, base)((9, None)).send(backend).body.toString.take(60))
    println(create.send(backend).body)
    println(interpreter.toRequest(Api.createPet, base)(("t-2", NewPet("x", Nil))).send(backend).body)
    println(interpreter.toRequest(Api.createPet, base)(("t-3", NewPet("x", Nil))).send(backend).body)
    println(me.send(backend).body)
    println(interpreter.toRequestThrowDecodeFailures(Api.count, base)(("cats", 3)).send(backend).body)
    println(interpreter.toRequestThrowErrors(Api.count, base)(("dogs", 1)).send(backend).body + 1)
    println(interpreter.toRequestThrowDecodeFailures(Api.getPet, base)((42, None)).send(backend).body)
    try interpreter.toRequestThrowErrors(Api.getPet, base)((7, None)).send(backend)
    catch case e: Exception => println(e.getClass.getSimpleName + ": " + e.getMessage.take(40))
