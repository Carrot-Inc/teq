package app

class LeanStdSuite extends munit.FunSuite:
  test("a Scala.js macro over the lean std's javalib"):
    assert(core.LeanStd.buildSbtExists)
    assert(!core.LeanStd.missingFileExists)
