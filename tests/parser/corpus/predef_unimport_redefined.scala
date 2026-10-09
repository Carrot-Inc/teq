// A name taken from Predef's root import by an explicit import and defined again: the program's
// own `assert` is the one called, and the wildcard keeps the rest of Predef.
import Predef.{assert as _, *}

object Checks:
  def assert(ok: Boolean): Unit = println(if ok then "checked" else "failed")

import Checks.assert

object Main:
  def main(args: Array[String]): Unit =
    assert(1 + 1 == 2)
    assert(false)
    println(Map("a" -> 1))
    require(true)
