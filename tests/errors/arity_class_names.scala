// The arity classes package `scala` holds by name: dotty's `ScalaPackageClass` synthesizes one only
// where `NameOps.isSyntheticFunction` holds (Definitions.scala:218-224, NameOps.scala:276-279), a
// `FunctionN` past `MaxImplementedFunctionArity` or a prefixed function name at any checked arity;
// any other name is what scala-library holds or nothing, and no tuple is made by its name. scalac
// 3.8.4, row by row: the review's probe LeadingZeros.scala whole, Tuple01 and Function01 not found
// (E008, "type Tuple01 is not a member of scala"); Function22 and Tuple22 scala-library's, taken;
// Function23 synthesized, taken; ContextFunction1 and ContextFunction01 synthesized at arity 1,
// taken; Tuple23, Tuple01 and Function01 not found (E008). The taken rows run in
// tests/cases/arity_class_names_found.scala. The probe's lambda has no parameter type: its
// AnonymousFunctionMissingParamType prints the erroneous expected type, so HideNonSensicalMessages hides it.
// expect: 22:10: error: type Tuple01 not found
// expect: 23:10: error: type Function01 not found
// expect: 31:12: error: type Tuple23 not found
// expect: 32:12: error: type Tuple01 not found
// expect: 33:12: error: type Function01 not found
// absent: 26:12: error
// absent: 27:12: error
// absent: 28:11: error
// absent: 29:12: error
// absent: 30:12: error
object LeadingZeros {
  val t: scala.Tuple01[Int] = Tuple1(1)
  val f: scala.Function01[Int, Int] = x => x
}
object Rows {
  val f22: scala.Function22[Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int] = ???
  val f23: scala.Function23[Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int] = ???
  val c1: scala.ContextFunction1[Int, Int] = ???
  val c01: scala.ContextFunction01[Int, Int] = ???
  val t22: scala.Tuple22[Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int] = ???
  val t23: scala.Tuple23[Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int] = ???
  val t01: scala.Tuple01[Int] = ???
  val f01: scala.Function01[Int, Int] = ???
}
// The pass on aeaf93410's rows, scalac 3.8.4's verdicts: a 23-element tuple typed before `scala.Tuple23` is
// named leaves that name not found (teq's own tuple class of the arity is no member of `scala`); `Function023`
// and `ContextFunction01` are classes of their own, no `Function23` or `ContextFunction1`
// (`Definitions.newFunctionNType` under the requested name); an arity name as a term is never synthesized, so
// `scala.Function23`, `scala.ContextFunction1`, `scala.Function2` and `scala.Tuple23` are no values, where
// scala-library's `object Tuple2` is.
object TupleLeak {
  val tuple = (0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22)
  val bogus: scala.Tuple23[Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int] = tuple
}
object FunctionIdentity {
  def bad(x: scala.Function023[Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int]): scala.Function23[Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int] = x
}
object Noncanonical {
  def bad(x: scala.ContextFunction01[Int, Int]): scala.ContextFunction1[Int, Int] = x
}
object Terms {
  val tuple2 = scala.Tuple2
  val function23 = scala.Function23
  val context1 = scala.ContextFunction1
  val function2 = scala.Function2
  val tuple23 = scala.Tuple23
}
// expect: 43:14: error: type Tuple23 not found
// expect: 46:293: error: Found:    (x : (Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int,
// expect: 49:85: error: Found:    (x : (Int) ?=> Int)
// expect: 12 errors found
// absent: 23:39: error
// expect: 53:20: error: value Function23 is not a member of scala
// expect: 54:18: error: value ContextFunction1 is not a member of scala
// expect: 55:19: error: value Function2 is not a member of scala
// expect: 56:17: error: value Tuple23 is not a member of scala
// absent: 52:16: error
// expect: Required: (Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int,
// expect: Required: Int
