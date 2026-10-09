// jars: scala-library
// std: scala-library
// targets: jvm
// The JVM target's linking: scala-library's bytecode as the JVM std.
// The builtin layer's own scala/Product, scala/Tuple2, scala/Function1 class files are
// emitted and, first on the class path, shadow scala-library's (Nil$.<clinit> then calls a
// Product.$init$ that teq's Product lacks).
@main def run(): Unit =
  val t = (1, "a")
  println(t.swap)
  println(List(1, 2).map(_ + 1))
