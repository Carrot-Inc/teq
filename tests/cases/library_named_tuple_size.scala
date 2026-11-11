//> using platform jvm
// jars: scala-library
// The exported inline forwarder is the first demand of the decomposition body.
// Its opaque owner's scope must not change the library match type's reduction.
@main def run(): Unit =
  val nt = (name = "a", count = 1)
  val size: 2 = nt.size
  println(size)
