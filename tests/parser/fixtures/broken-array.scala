// An overload that fails inside a generator over an array, and inside a lambda over one: the
// element type the failure leaves is no ClassTag's to search for, as scalac's erroneous argument is
// not.
@main def run(): Unit =
  println((for x <- Array(1, 2, 3) if x != 2; y <- List(x, x * "s") yield y).mkString(","))
  println(Array(1, 2).flatMap(x => List(x, x * "s").map(y => y)).mkString(","))
