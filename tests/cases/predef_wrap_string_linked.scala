// jars: scala-library
// With scala-library's jar on the class path, its `Predef` is the one a program names, and the
// body of its `wrapString` runs against the standard library: `ScalaRunTime.mapNull` is there too.
@main def run(): Unit =
  println(scala.Predef.wrapString("abc").toList)
  println(Predef.wrapString("xyz").length)
  val s: String = null
  println(scala.Predef.wrapString(s) == null)
