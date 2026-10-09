// jars: scala-library tapir-core tapir-client tapir-sttp-client4 tapir-json-zio zio-json magnolia123 zio sttp-client4-core sttp-model sttp-shared-core sttp-shared-ws scala-java-time
//> using platform js
//> using dep com.softwaremill.sttp.tapir::tapir-sttp-client4::1.13.29
//> using dep com.softwaremill.sttp.tapir::tapir-json-zio::1.13.29
//> using dep dev.zio::zio-json::0.9.2
// tapir 1.13.29's sttp client interpreter over sttp's `FetchBackend`, both from their Scala.js
// jars, with the global `fetch` replaced by one answering canned JSON under node: the requests
// `toRequest` and `toSecureRequest` build from endpoints with a path capture, a query, a
// header, a JSON body and a bearer token as `fetch` receives them, and the responses decoded
// through the endpoints' outputs, an error output on a 404 and a failed decoding among them.
package tapirfetch

import scala.scalajs.js
import scala.concurrent.Future
import scala.concurrent.ExecutionContext.Implicits.global
import sttp.client4.fetch.FetchBackend
import sttp.model.StatusCode
import sttp.tapir.*
import sttp.tapir.client.sttp4.SttpClientInterpreter
import sttp.tapir.json.zio.*
import sttp.tapir.generic.auto.*
import zio.json.*

case class Pet(id: Int, name: String, tags: List[String]) derives JsonCodec, Schema
case class NewPet(name: String, tags: List[String]) derives JsonCodec, Schema

object Api:
  val getPet: PublicEndpoint[(Int, Option[String]), String, Pet, Any] =
    endpoint.get.in("pets" / path[Int]("id")).in(query[Option[String]]("expand")).out(jsonBody[Pet]).errorOut(stringBody)
  val addPet: Endpoint[String, (NewPet, String), Unit, Pet, Any] =
    endpoint.post.securityIn(auth.bearer[String]()).in("pets").in(jsonBody[NewPet]).in(header[String]("X-Trace"))
      .out(statusCode(StatusCode.Created).and(jsonBody[Pet]))

object Main:
  def headersOf(h: js.Dynamic): String =
    js.Array.from[js.Array[String]](h.asInstanceOf[js.Iterable[js.Array[String]]]).map(kv => kv(0) + "=" + kv(1)).sorted.mkString(",")

  def answer(url: String, method: String, body: String): (Int, String) =
    if url.contains("/pets/404") then (404, "no such pet")
    else if url.contains("/pets/500") then (200, "{\"id\":\"x\"}")
    else if method == "POST" then (201, body.dropRight(1) + ",\"id\":7}")
    else (200, "{\"id\":1,\"name\":\"rex\",\"tags\":[\"good\"]}")

  def main(args: Array[String]): Unit =
    js.Dynamic.global.updateDynamic("fetch")((req: js.Dynamic) => {
      val url = req.url.asInstanceOf[String]
      val method = req.method.asInstanceOf[String]
      req.text().`then`((body: String) => {
        println("fetch " + method + " " + url + " " + headersOf(req.headers) + " [" + body + "]")
        val (status, text) = answer(url, method, body)
        val init = js.Dynamic.literal(status = status, headers = js.Dynamic.literal("Content-Type" -> "application/json"))
        js.Dynamic.newInstance(js.Dynamic.global.Response)(text, init)
      })
    })
    val backend = FetchBackend()
    val client = SttpClientInterpreter()
    val base = Some(sttp.model.Uri.unsafeParse("https://api.example.com"))
    val steps: List[() => Future[String]] = List(
      () => client.toRequest(Api.getPet, base).apply((1, Some("owner"))).send(backend).map(r => r.code.toString + " " + r.body),
      () => client.toRequest(Api.getPet, base).apply((404, None)).send(backend).map(r => r.code.toString + " " + r.body),
      () => client.toRequest(Api.getPet, base).apply((500, None)).send(backend).map(r =>
        r.code.toString + " " + r.body.getClass.getSimpleName + " " + r.body.isInstanceOf[DecodeResult.Failure]),
      () => client.toSecureRequest(Api.addPet, base).apply("tok").apply((NewPet("fido", List("new")), "t-1")).send(backend)
        .map(r => r.code.toString + " " + r.body),
      () => Future.successful(client.toRequestThrowDecodeFailures(Api.getPet, base).apply((2, None)).uri.toString)
    )
    steps.foldLeft(Future.successful(())) { (done, step) =>
      done.flatMap(_ => step().map(println))
    }.onComplete(t => println("finished " + t.isSuccess))
