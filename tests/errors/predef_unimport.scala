// expect: 13:5: error: not found: assert
// expect: 14:5: error: not found: println
// expect: 2 errors found
// An explicit import of `Predef` takes its root import away below it, as under scalac: only
// what its selectors name is imported, so an exclusion alone leaves none of Predef's members.
object Before:
  def run(): Unit = println("before")

import Predef.{assert as _}

object Test:
  def run(): Unit =
    assert(1 == 1)
    println("x")
    val xs = List(1, 2)
