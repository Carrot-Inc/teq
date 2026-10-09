// expect: reference to a is ambiguous: it is both imported by import Q.* and imported subsequently by import R.*
// expect: reference to b is ambiguous: it is both imported by name by import Q.b and imported by name subsequently by import R.b
// expect: reference to x is ambiguous: it is both defined in object T and imported subsequently by import P.x
// expect: a is imported twice on the same import line
// expect: named imports cannot follow wildcard imports
object Q:
  val a = "Q"
  val b = "Q"
object R:
  val a = "R"
  val b = "R"
object P:
  val x = "P"

object Test:
  import Q.*
  import R.*
  def v = a
object Test2:
  import Q.b
  import R.b
  def v = b
object T:
  val x = "T"
  def f =
    import P.x
    x

import Q.{a, a}
import Q.{*, b}

@main def run(): Unit = println(Test.v)
