// jars: predef-lib
// A jar's body that reads a repeated parameter as a value, ascribed with the repeated type: the
// sequence of the arguments.
@main def run(): Unit =
  println(predeflib.Multis.partsOf(new predeflib.Multi("a", "b")))
  println(predeflib.Multis.partsOf("c"))
