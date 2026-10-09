// jars: sttp-client4-core sttp-client4-zio sttp-model sttp-shared-core sttp-shared-ws sttp-shared-zio zio zio-streams zio-stacktracer zio-internal-macros izumi-reflect izumi-reflect-boopickle scala-collection-compat macrotask-executor scala-java-time
//> using platform js
//> using dep com.softwaremill.sttp.client4::zio::4.0.26
// sttp-model's `Uri.toJavaUri` names
// `java.net.URI`, which the std's platform layer has as Scala.js's javalib does.
// sttp client4 4.0.26's `FetchZioBackend` from its Scala.js jar with zio 2.1.26's runtime, over
// the global `fetch`, which the program replaces with one answering canned responses under
// node: requests sent as `Task`s through `RIOMonadAsyncError`, the responses read back, a 404
// as a `Left`, a rejected `fetch` as a failed effect, and a streamed response body.
import scala.scalajs.js
import sttp.client4.*
import sttp.client4.impl.zio.FetchZioBackend
import sttp.capabilities.zio.ZioStreams
import zio.*
import zio.stream.*

object Main extends ZIOAppDefault:
  def respond(url: String): js.Dynamic =
    val (status, text) = if url.contains("missing") then (404, "no pet") else (200, "pet:" + url.split('/').last)
    js.Dynamic.newInstance(js.Dynamic.global.Response)(text, js.Dynamic.literal(status = status))

  def run =
    js.Dynamic.global.updateDynamic("fetch")((req: js.Dynamic) => {
      val url = req.url.asInstanceOf[String]
      println("fetch " + req.method + " " + url)
      if url.contains("down") then js.Dynamic.global.Promise.reject(js.Dynamic.newInstance(js.Dynamic.global.TypeError)("Failed to fetch"))
      else js.Dynamic.global.Promise.resolve(respond(url))
    })
    val backend = FetchZioBackend()
    for
      r1 <- basicRequest.get(uri"https://example.com/pets/1").send(backend)
      _ <- Console.printLine(r1.code.toString + " " + r1.body)
      r2 <- basicRequest.get(uri"https://example.com/missing").send(backend)
      _ <- Console.printLine(r2.code.toString + " " + r2.body)
      r3 <- basicRequest.get(uri"https://example.com/down").send(backend).either
      _ <- Console.printLine(r3.left.map(_.getClass.getSimpleName).toString)
      r4 <- basicRequest.get(uri"https://example.com/pets/stream").response(asStreamAlwaysUnsafe(ZioStreams)).send(backend)
      bytes <- r4.body.runCollect
      _ <- Console.printLine(r4.code.toString + " " + new String(bytes.toArray, "UTF-8"))
    yield ()
