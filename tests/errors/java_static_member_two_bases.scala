// jars: scala-library
// teq: --target jvm
// Two Java bases each with a static member class `Builder` (java.util.Calendar's class, java.util.stream.Stream's
// interface): neither is a member of the class below them, and the name is not found, as scalac 3.8.4 says
// (E006).
// expect: 8:13: error: type Builder not found
abstract class Odd extends java.util.Calendar with java.util.stream.Stream[String]:
  def made: Builder = null

@main def run(): Unit =
  println(1)
