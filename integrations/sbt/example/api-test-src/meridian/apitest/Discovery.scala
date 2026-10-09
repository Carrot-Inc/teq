package meridian.apitest

import org.scalatest.funsuite.AnyFunSuite
import org.scalatest.matchers.should.Matchers

/** The suites `definedTests` must and must not find: a base extending the framework's suite is
  * abstract and no test, a concrete class extending the base is one through the base, a
  * private one is not public and no test. */
abstract class ApiFunSuite extends AnyFunSuite with Matchers:
  def subject: String = "api"

class IndirectSuite extends ApiFunSuite:
  test("the subject is set") {
    subject shouldBe "api"
  }

private class HiddenSuite extends AnyFunSuite:
  test("never discovered") {
    assert(false)
  }
