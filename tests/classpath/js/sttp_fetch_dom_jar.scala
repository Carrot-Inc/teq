// jars: scala-library sttp-client4-core sttp-model sttp-shared-core sttp-shared-ws scala-java-time scalajs-dom
// sttp_fetch.scala with scalajs-dom on the class path, whose facades then replace the std's DOM
// shapes: the backend's `Headers` iterated through `@JSName(js.Symbol.iterator)`, the jar's
// `RequestInit` a JS trait whose vars come with their setters.
//> using platform js
//> using dep com.softwaremill.sttp.client4::core::4.0.26
// sttp client4 4.0.26's `FetchBackend` from its Scala.js jar over the global `fetch`, which the
// program replaces with one answering canned responses under node: the `Request` the backend
// builds (method, URL, headers, redirect mode, string, byte and multipart bodies) as `fetch`
// receives it, the responses read back as strings and bytes with their codes and headers, a
// rejected `fetch` as an sttp exception, and a read timeout set and cleared, in sequence.
import scala.scalajs.js
import scala.concurrent.Future
import scala.concurrent.duration.*
import scala.concurrent.ExecutionContext.Implicits.global
import sttp.client4.*
import sttp.client4.fetch.FetchBackend
import sttp.model.{MediaType, StatusCode}

object Main:
  def headersOf(h: js.Dynamic): String =
    js.Array.from[js.Array[String]](h.asInstanceOf[js.Iterable[js.Array[String]]]).map(kv => kv(0) + "=" + kv(1)).sorted.mkString(",")

  def respond(req: js.Dynamic, body: String): js.Dynamic =
    val url = req.url.asInstanceOf[String]
    val init = js.Dynamic.literal(
      status = if url.contains("missing") then 404 else 200,
      statusText = if url.contains("missing") then "Not Found" else "OK",
      headers = js.Dynamic.literal("X-Kind" -> "canned", "Content-Type" -> "text/plain")
    )
    js.Dynamic.newInstance(js.Dynamic.global.Response)(if url.contains("missing") then "no pet" else "echo:" + body, init)

  def main(args: Array[String]): Unit =
    js.Dynamic.global.updateDynamic("fetch")((req: js.Dynamic) => {
      val url = req.url.asInstanceOf[String]
      val ct = req.headers.get("content-type")
      val kind = if ct != null && ct.asInstanceOf[String].startsWith("multipart/form-data") then "multipart" else "plain"
      println("fetch " + req.method + " " + url + " redirect=" + req.redirect + " " + kind)
      if url.contains("down") then js.Dynamic.global.Promise.reject(js.Dynamic.newInstance(js.Dynamic.global.TypeError)("Failed to fetch"))
      else req.text().`then`((body: String) => {
        if kind == "multipart" then println("  parts " + body.contains("name=\"a\"") + " " + body.contains("filename=\"f.txt\""))
        else println("  headers " + headersOf(req.headers) + " body [" + body + "]")
        respond(req, if kind == "multipart" then "parts" else body)
      })
    })
    val backend = FetchBackend()
    val base = uri"https://example.com/api/pets/42?name=rex fido&tag=a"
    val steps: List[() => Future[String]] = List(
      () => basicRequest.get(base).header("X-Trace", "abc").send(backend).map(r =>
        r.code.toString + " " + r.body + " " + r.statusText + " " + r.header("X-Kind") + " " + r.contentType),
      () => basicRequest.post(uri"https://example.com/pets").body("""{"name":"rex"}""").contentType(MediaType.ApplicationJson)
        .send(backend).map(r => r.code.toString + " " + r.body),
      () => basicRequest.put(uri"https://example.com/raw").body(Array[Byte](104, 105)).response(asByteArray)
        .send(backend).map(r => r.code.toString + " " + r.body.map(_.toList)),
      () => basicRequest.get(uri"https://example.com/missing").send(backend).map(r =>
        r.code.toString + " " + r.body + " " + r.isClientError + " " + (r.code == StatusCode.NotFound)),
      () => basicRequest.get(uri"https://example.com/nofollow").followRedirects(false).readTimeout(5.seconds)
        .response(asStringAlways).send(backend).map(r => r.code.toString + " " + r.body),
      () => basicRequest.post(uri"https://example.com/upload")
        .multipartBody(multipart("a", "1"), multipart("f", "data").fileName("f.txt"))
        .send(backend).map(r => r.code.toString + " " + r.body),
      () => basicRequest.get(uri"https://example.com/down").send(backend).map(_ => "no failure")
        .recover { case e: SttpClientException => "sttp " + e.getClass.getSimpleName; case e => "other " + e.getClass.getSimpleName + " " + e.getMessage }
    )
    steps.foldLeft(Future.successful(())) { (done, step) =>
      done.flatMap(_ => step().map(println))
    }.onComplete(t => println("finished " + t.isSuccess))
