package jvb

@main def run(): Unit =
  val xs = new Array[String](3)
  xs(0) = "a"
  xs(1) = "b"
  xs(2) = "c"
  println(jva.Words.last)
  println(xs.lastOption)
  println(xs.map(_.toUpperCase).mkString(","))
  println("ab1".forall(_.isLetterOrDigit))
  // scala-library's opaque `IArray`, whose companion its jar nests in `IArray$package`.
  println(IArray.from(List(4, 5)).length)
