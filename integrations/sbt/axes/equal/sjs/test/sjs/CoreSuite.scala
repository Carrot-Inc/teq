package sjs

class CoreSuite extends munit.FunSuite {
  test("the core's number is one") { assertEquals(Core.n, 1) }
  test("the core's inline method doubles") { assertEquals(Core.twice(2), 4) }
}
