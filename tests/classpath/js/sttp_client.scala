// jars: scala-library sttp-client4-core sttp-model sttp-shared-core sttp-shared-ws scala-java-time
//> using dep com.softwaremill.sttp.client4::core:4.0.26
// sttp client4 4.0.26 from its jar: `basicRequest` with `uri"..."` interpolation, `addParam`,
// bodies, headers, `response(asString)`, `mapResponse`, `Request.show`, `StatusCode`, `Uri`'s
// members, and a synchronous `BackendStub` answering canned responses so the requests run
// without a network on both platforms.
package sttpclient

import sttp.client4.*
import sttp.client4.testing.{BackendStub, ResponseStub}
import sttp.model.{Header, HeaderNames, MediaType, StatusCode, Uri}

object Main:
  def main(args: Array[String]): Unit =
    val id = 42
    val name = "rex fido"
    val base = uri"https://example.com/api/pets/$id?name=$name&tag=a&tag=b"
    println(base.toString)
    println(base.host.toString + " " + base.port + " " + base.scheme + " " + base.path + " " + base.paramsMap + " " + base.params.getMulti("tag"))
    println(base.addParam("page", "2").toString)
    println(base.withPath("v2", "pets").toString + " " + base.addPath("photos").toString)
    println(Uri.unsafeParse("http://localhost:8080/a/b?x=1").toString + " " + Uri.parse("::bad::").isLeft)
    println(uri"https://example.com/search?q=${Some("scala 3")}&empty=${None}".toString)
    println(uri"https://example.com".withParams(Map("a" -> "1", "b" -> "2")).toString + " " + uri"http://x.org/p".pathSegments.segments.map(_.v).toList)

    val get = basicRequest.get(base).header("X-Trace", "abc").header(Header.accept(MediaType.ApplicationJson))
    println(get.show())
    println(get.showBasic + " | " + get.method + " " + get.headers.map(h => h.name + "=" + h.value).mkString(","))
    val post = basicRequest.post(uri"https://example.com/pets").body("""{"name":"rex"}""").contentType(MediaType.ApplicationJson).auth.bearer("tok")
    println(post.show(includeBody = true, includeHeaders = true))
    println(post.body.toString + " " + post.header(HeaderNames.ContentType) + " " + post.header(HeaderNames.Authorization))
    val bytes = basicRequest.put(uri"https://example.com/raw").body(Array[Byte](1, 2, 3)).response(asByteArray)
    println(bytes.show() + " " + bytes.body.getClass.getSimpleName)

    val backend = BackendStub.synchronous
      .whenRequestMatches(r => r.uri.path.endsWith(List("pets", "42")))
      .thenRespondAdjust("""{"id":42,"name":"rex"}""")
      .whenRequestMatches(r => r.method == sttp.model.Method.POST)
      .thenRespondAdjust("created", StatusCode.Created)
      .whenRequestMatches(r => r.uri.path.contains("missing"))
      .thenRespondNotFound()
      .whenAnyRequest
      .thenRespond(ResponseStub.adjust("fallback", StatusCode.Ok, List(Header("X-Kind", "any"))))

    val r1 = get.response(asString).send(backend)
    println(r1.code.toString + " " + r1.body + " " + r1.isSuccess + " " + r1.statusText)
    val r2 = post.response(asStringAlways).send(backend)
    println(r2.code.toString + " " + r2.body + " " + r2.code.isSuccess + " " + r2.code.code)
    val r3 = basicRequest.get(uri"https://example.com/missing").response(asString).send(backend)
    println(r3.code.toString + " " + r3.body + " " + r3.isClientError + " " + StatusCode.NotFound.toString)
    val r4 = basicRequest.get(uri"https://example.com/other").response(asStringAlways.map(_.toUpperCase)).send(backend)
    println(r4.body + " " + r4.header("X-Kind") + " " + r4.headers.map(_.name).toList)
    val r5 = basicRequest.get(uri"https://example.com/other").response(asString.mapRight((s: String) => s.length)).send(backend)
    println(r5.body.toString)
    val r6 = basicRequest.get(uri"https://example.com/other").response(asStringOrFail).mapResponse(_.take(3)).send(backend)
    println(r6.body)
    try basicRequest.get(uri"https://example.com/missing").response(asStringOrFail).send(backend)
    catch case e: SttpClientException => println(e.getClass.getSimpleName + ": " + e.getMessage + " / " + e.getCause.getMessage)
    val either = basicRequest.get(uri"https://example.com/pets/42").response(asString.map(_.map(_.length))).send(backend)
    println(either.body.toString + " " + either.request.method)
    println(StatusCode.Ok.toString + " " + StatusCode(418).toString + " " + StatusCode.InternalServerError.isServerError + " " + StatusCode.Found.isRedirect)
    println(MediaType.ApplicationJson.toString + " " + MediaType.parse("text/plain; charset=utf-8").map(_.charset).toString + " " + MediaType.TextPlainUtf8.matches(sttp.model.ContentTypeRange.AnyText))
    println(Header.contentType(MediaType.ApplicationJson).toString + " " + Header.contentLength(3) + " " + HeaderNames.Accept)
