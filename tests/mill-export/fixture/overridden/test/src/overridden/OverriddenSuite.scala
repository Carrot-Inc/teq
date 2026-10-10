package overridden

class OverriddenSuite extends munit.FunSuite:
  test("one")(assertEquals(Overridden.n, 1))
