package meridian.apitest

import meridian.core.http.{Method, Request}
import org.scalatest.funsuite.AnyFunSuite
import org.scalatest.matchers.should.Matchers

class ScalatestSuite extends AnyFunSuite with Matchers:
  test("a request keeps its path") {
    val request = Request(Method.Get, List("v1", "sites"), Nil, Nil, None)
    request.segments should contain("sites")
    request.segments.length shouldBe 2
    assert(request.render.startsWith("Get /"))
  }
