package fixture

import utest.*

object UtestSuite extends TestSuite:
  val tests = Tests {
    test("passes") { assert(Core.twice(4) == 8) }
    test("fails") { assert(Core.twice(4) == 9) }
    test("group") {
      test("nested") { assert(Core.twice(5) == 10) }
    }
  }
