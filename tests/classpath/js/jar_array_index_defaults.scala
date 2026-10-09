// jars: predef-lib
// A jar's `indexWhere` and `lastIndexWhere` of an array, which pass the default of the bound: the
// std's `indexWhere` has no such parameter and does what the default does.
@main def run(): Unit =
  println(predeflib.ArrayScans.firstAndLast(Array(1, 2, 3, 1)))
  println(predeflib.ArrayScans.firstAndLast(Array(0)))
