// Adapted from scala3 tests/neg/i0281-null-primitive-conforms.scala and tests/neg/null-anyval.scala (Apache-2.0, see tests/scala3/README.md); the AnyVal cases are left out.
// expect: type mismatch: found Null, required Boolean
// expect: type mismatch: found Null, required Double
// expect: type mismatch: found Int | Null, required Int
// expect: type mismatch: found Null, required T

object test:
  val b: scala.Boolean = null
  val c: Unit = null
  val d: Double = null

object Test:
  val x: Int = 0
  val y: Int | Null = x
  val z2: Int = y
  val z4: O.T = null

object O:
  opaque type T = String
