// The arity classes package `scala` gives by name where scalac 3.8.4 takes them: Function22 and
// Tuple22 scala-library's, Function23 synthesized past `MaxImplementedFunctionArity`, ContextFunction1
// and ContextFunction01 synthesized at arity 1 (`NameOps.isSyntheticFunction`, Definitions.scala:221).
// The names scalac does not find are tests/errors/arity_class_names.scala.
package arity_class_names_found

@main def main(): Unit =
  val f22: scala.Function22[Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int] = (a1, a2, a3, a4, a5, a6, a7, a8, a9, a10, a11, a12, a13, a14, a15, a16, a17, a18, a19, a20, a21, a22) => a1 + a22
  println(f22(1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22))
  val f23: scala.Function23[Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int] = (a1, a2, a3, a4, a5, a6, a7, a8, a9, a10, a11, a12, a13, a14, a15, a16, a17, a18, a19, a20, a21, a22, a23) => a1 + a23
  println(f23(1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23))
  val c1: scala.ContextFunction1[Int, Int] = (x: Int) ?=> x + 1
  println(c1(using 1))
  val c01: scala.ContextFunction01[Int, Int] = (x: Int) ?=> x + 2
  println(c01(using 1))
  val t22: scala.Tuple22[Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int] = (1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22)
  println(t22._22)
