package fixture

import scala.scalajs.js

class CjsSuite extends munit.FunSuite:
  test("the environment's variable") {
    assertEquals(js.Dynamic.global.process.env.FIXTURE_GREETING.asInstanceOf[String], "hello")
  }
  test("node's argument") {
    assert(js.Dynamic.global.process.execArgv.asInstanceOf[js.Array[String]].contains("--max-old-space-size=300"))
  }
