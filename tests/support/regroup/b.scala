// A library compiled against tests/support/regroup/a_v1.scala and run with version 2: its call
// `check(a, b, clue)(loc, compare)` names by its signature the method version 2 keeps for binary
// compatibility, which takes the five arguments in one clause (munit-cats-effect's `assertIO` over munit
// 1.2).
package rgb

import rga.*

object Client:
  def run(a: Int, b: Int): String =
    given Loc = Loc(7)
    Assertions.check(a, b, "client")
