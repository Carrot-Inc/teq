package fixture

import scala.concurrent.Future

val Slow = new munit.Tag("Slow")

class MunitSuite extends munit.FunSuite:
  test("passes") { assertEquals(Core.twice(21), 42) }
  test("passes asynchronously") { Future.successful(assertEquals(Core.twice(2), 4)) }
  test("is ignored".ignore) { fail("never run") }
  test("fails") { assertEquals(Core.twice(2), 5) }
  test("is slow".tag(Slow)) { fail("left out by the framework's argument") }

// Discovered through its parent; the abstract parent is not.
abstract class MunitBase extends munit.FunSuite:
  test("inherited") { assertEquals(Core.twice(1), 2) }

class MunitInherited extends MunitBase

// Named by the build's Tests.Exclude.
class MunitExcluded extends munit.FunSuite:
  test("never run") { fail("excluded") }

// A class nested in an object is no suite of sbt's discovery.
object Holder:
  class Nested extends munit.FunSuite:
    test("never discovered") { fail("nested") }
