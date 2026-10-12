// `##` of a Float is `Statics.floatHash`: a whole value hashes as the Int or Long it equals (master: `Float.hashCode`
// on the JVM, the Int step alone in the interpreter); any other as `Float.hashCode`, which JavaScript's floats have not.
@main def run(): Unit =
  val one = 1.0f
  println(one.##)
  println((1L << 40).toFloat.##)
  println((one: Any).##)
  println(((1L << 40).toFloat: Any).##)
  println((-3.0f).##)
  println((1L << 40).toDouble.##)
  println(Set[Any](1.0f, 1, 1.0).size)
