package meridian.apitest

import munit.FunSuite

// `Util.value` is an inline constant of api/lib/util.jar: a jar rebuilt in place reaches the
// suite only through a resident started afresh.
class JarSuite extends FunSuite:
  test("the jar's inline constant") {
    assertEquals(util.Util.value, 1)
  }
