// jars: predef-lib
// A jar's for comprehension over an array with a guard, which maps its `withFilter` with the
// `ClassTag` of the result.
@main def run(): Unit =
  val out = predeflib.ArrayFors.evens(Array(1, 2, 3, 4))
  println(out.mkString(" "))
  println(out.length)
