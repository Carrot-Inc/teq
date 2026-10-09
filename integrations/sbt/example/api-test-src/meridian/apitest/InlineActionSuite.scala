package meridian.apitest

import munit.FunSuite

// `Actions.make()` is an inline def of the main sources whose body makes an anonymous class:
// under `--own api-test-src` that class is written with this suite's class files, since the
// main build cannot supply a class made for this call site.
class InlineActionSuite extends FunSuite:
  test("an anonymous class made by an inline expansion of the main sources") {
    assertEquals(meridian.check.Actions.make().run(), 42)
  }
