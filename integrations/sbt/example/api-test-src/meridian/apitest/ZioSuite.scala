package meridian.apitest

import meridian.core.http.{Method, Request}
import zio.test.*

object ZioSuite extends ZIOSpecDefault:
  def spec = suite("zio-test")(
    test("a request renders its path") {
      val request = Request(Method.Get, List("v1", "sites"), Nil, Nil, None)
      assertTrue(request.render == "Get /v1/sites", request.segments.length == 2)
    },
    test("effects run") {
      for n <- zio.ZIO.succeed(21) yield assertTrue(n * 2 == 42)
    },
  )
