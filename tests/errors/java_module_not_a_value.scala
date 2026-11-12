// jars: javafix
// The module of a JDK class the std stands for, which a term naming it reaches with a class path, is a Java module
// (`moduleRoot.setFlag(JavaDefined | ...)`, ClassfileParser.scala:398), so it is no value (`isNoValue`, `checkValue`):
// teq is to report it as scalac 3.8.4 does, for the std's `trait Future` and its `object Map`'s `trait Entry`.
// expect: error: Java defined class Future is not a value
// expect: error: Java defined class Entry is not a value
@main def run(): Unit =
  val f = java.util.concurrent.Future
  val e = java.util.Map.Entry
  println(f != null && e != null)
