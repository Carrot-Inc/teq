package web

import utest.*

object WebTests extends TestSuite:
  val tests = Tests {
    test("name")(assert(Web.name == "web"))
  }
