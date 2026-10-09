// expect: 26:11: error: reference to a is ambiguous: it is both imported by import Q.* and imported subsequently by import R.*
// expect: 30:11: error: reference to b is ambiguous: it is both imported by name by import Q.b and imported by name subsequently by import R.b
// expect: 35:5: error: reference to x is ambiguous: it is both defined in object T and imported by name subsequently by import P.x
// expect: 38:5: error: reference to x is ambiguous: it is both defined in an enclosing scope and imported by name subsequently by import P.x
// expect: 43:5: error: reference to a is ambiguous: it is both imported by name by import Q.a and imported subsequently by import R.*
// expect: 47:11: error: reference to top is ambiguous: it is both defined in package <empty> and imported subsequently by import P.*
// expect: 48:8: error: s1 is imported twice on the same import line
// expect: 49:8: error: named imports cannot follow wildcard imports
// expect: 8 errors found
object Q:
  val a = "Q"
  val b = "Q"
object R:
  val a = "R"
  val b = "R"
object P:
  val x = "P"
  val top = "P.top"
object S:
  val s1 = 1
  val s2 = 2

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
  def g(x: Int) =
    import P.x
    x
object Outer:
  import Q.a
  def f =
    import R.*
    a
val top = "top"
object U:
  import P.*
  def f = top
import S.{s1, s1}
import S.{*, s2}
object Fine:
  import Q.*
  def f =
    import R.a
    a
  def g =
    import R.*
    a
  def h =
    import Q.a
    a
