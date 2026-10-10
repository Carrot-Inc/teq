package app

class AppSuite extends munit.FunSuite {
  test("the core's number doubled") { assertEquals(App.doubled, 2) }
}
