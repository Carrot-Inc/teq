// jars: scala-library
// std: scala-library
// A literal written where a wider number is expected is that number in a varargs sequence
// too: scala-library's `Array.apply(x: Double, xs: Double*)` gets `Double`s for `Array(1.5, 2)`,
// as it gets one for `Array(2, 1.5)`, and the array a type expects is made of what it holds.
@main def run(): Unit =
  println(Array(1.5, 2).mkString(","))
  println(Array(2, 1.5).mkString(","))
  println(Array(1.5, 2, 3).mkString(","))
  val i = 1
  println(Array(i, 2.5).mkString(","))
  val longs: Array[Long] = Array(1, 2)
  val doubles: Array[Double] = Array(1, 2)
  println(longs.mkString(",") + " " + doubles.mkString(",") + " " + longs.getClass.getSimpleName + " " + doubles.getClass.getSimpleName)
  val floats: Seq[Float] = Seq(1, 2.5f)
  println(floats.mkString(",") + " " + Vector[Double](1, 2).mkString(","))
