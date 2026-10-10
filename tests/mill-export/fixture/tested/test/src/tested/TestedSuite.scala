package tested

class TestedSuite extends munit.FunSuite:
  test("one")(assertEquals(Tested.n, Helper.n()))
