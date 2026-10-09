package app

class CoreSuite extends munit.FunSuite:
  test("the core's number is one") {
    assertEquals(core.Core.n, 1)
  }
  test("the core's inline method doubles") {
    assertEquals(core.Core.twice(2), 4)
  }
