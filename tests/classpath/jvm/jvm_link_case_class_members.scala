// jars: scala-library
// std: scala-library
// targets: jvm
// The JVM target's linking: scala-library's bytecode as the JVM std.
// Product members scala-library calls on a case class of the program: canEqual (an
// AbstractMethodError before) and productElementName (productElementNames gave List(, )).
case class P(x: Int, name: String)
@main def run(): Unit =
  val p = P(1, "a")
  val e: Product = p
  println(e.canEqual(P(2, "b")))
  println(p.productIterator.toList)
  println(p.productElementNames.toList)
