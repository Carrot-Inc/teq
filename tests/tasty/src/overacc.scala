package fix.overacc

// Two inline methods of one name whose bodies reach two private overloads of one name: scalac's inline accessors
// `inline$fix$overacc$AccAssertions$$checkImpl` of two arities, which each body selects by its signature (scalatest's
// `assertResult` over its two `assertResultImpl`).
trait AccAssertions:
  inline def check[L, R](expected: L, clue: Any)(actual: R): String = checkImpl[L, R](expected, clue, actual)
  private final def checkImpl[L, R](expected: L, clue: Any, actual: R): String = s"3 $expected $clue $actual"
  inline def check[L, R](expected: L)(actual: R): String = checkImpl[L, R](expected, actual)
  private final def checkImpl[L, R](expected: L, actual: R): String = s"2 $expected $actual"
