// jars: scala-library defaults-lib
// A jar's export of a case class's `apply` (tests/support/defaults_lib.scala's `PtApply`) has its getters under `<init>`'s
// name (Namer.scala:1249-1262), and a call's default is the getter of the method's own name (`findDefaultGetter`,
// Applications.scala:382-402): the omitted argument is missing. scalac 3.8.4: E171 at the call, its result type
// `defaultslib.Pt`, including its package even though that package is imported below.
// expect: 11:24: error: missing argument for parameter x of method apply in object PtApply: (x: Int, y: Int): defaultslib.Pt
// expect: 1 error found
import defaultslib.*

@main def main(): Unit =
  println(PtApply.apply(y = 5))
