package meridian.apitest

import meridian.core.http.{Method, Request}
import munit.ScalaCheckSuite
import org.scalacheck.Prop.forAll

class MunitSuite extends ScalaCheckSuite:
  test("a request renders its method") {
    assertEquals(Request(Method.Get, List("v1"), Nil, Nil, None).render.takeWhile(_ != ' '), "Get")
  }

  property("reversing a list twice gives it back") {
    forAll { (xs: List[Int]) => xs.reverse.reverse == xs }
  }
