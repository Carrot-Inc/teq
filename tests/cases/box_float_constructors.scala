// `java.lang.Float`'s three constructors, `(float)`, `(double)` and `(String)`, each sent to the
// companion method of its signature (Scala.js's `JSCodeGen.genNewHijackedClass`); the double one
// rounds to the nearest float, the string one parses as `Float.parseFloat`.
@main def run(): Unit =
  println(new java.lang.Float("1.5").floatValue())
  println(new java.lang.Float(" -2.25 ").doubleValue())
  val f = new java.lang.Float(16777217.0)
  println(f.intValue())
  println(f.doubleValue() == 16777216.0)
  println(new java.lang.Float(0.1f).doubleValue() == 0.1f.toDouble)
  println(java.lang.Float.parseFloat("3.4028236E38").isInfinite)
  try new java.lang.Float("x") catch case e: NumberFormatException => println("bad")
