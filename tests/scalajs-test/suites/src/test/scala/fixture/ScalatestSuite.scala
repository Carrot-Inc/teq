package fixture

import org.scalatest.funsuite.AnyFunSuite

class ScalatestSuite extends AnyFunSuite:
  test("passes") { assert(Core.twice(3) == 6) }
  test("fails") { assert(Core.twice(3) == 7) }
  ignore("is ignored") { fail("never run") }
  test("is pending") { pending }
  test("is canceled") { cancel("not now") }

// Discovered, but the framework cannot make it: never a suite left out silently.
class ScalatestNeedsArgument(n: Int) extends AnyFunSuite:
  test("never made") { assert(n == n) }
