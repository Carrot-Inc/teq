package lib

class UtilSuite extends munit.FunSuite {
  test("twice doubles") { assertEquals(Util.twice(2), 4) }
  test("the generated word") { assertEquals(Util.greeting, "hello, 42") }
}
