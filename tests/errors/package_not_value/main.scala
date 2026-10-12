// expect: main.scala:13:11: error: views is a package, not a value
// expect: main.scala:14:8: error: views is a package, not a value
// expect: 2 errors found
// absent: main.scala:12:
// A package with a top-level `apply` is applied through it, but is no value where it is not applied
// (dotty's checkValue leaves only a FunOrPolyProto unchecked).
package app

def take(a: Any): Any = a

@main def main(): Unit =
  println(views.activity(1))
  val v = views.activity
  take(views.activity)
