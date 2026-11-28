class jvm(template: String) extends scala.annotation.StaticAnnotation
@jvm("$0 ineg") def negate(s: String): Int = 3
@jvm("$0:I array_new") def allocate(n: Int): AnyRef = "array"

object Probe:
  def main(args: Array[String]): Unit =
    println(allocate(1))
    println(negate("x"))

// The creation of an array with an unknown element type precedes the negation of a String.
// scalac 3.8.4 builds it and prints `array` then `3`.
// command: build
// teq: --target jvm --all-mains
// jars: scala-library
// exit: 3
// expect: jvm backend: Probe$.main: a new array of an element type only a ClassTag knows
// expect: jvm backend: Probe$.main: the instruction ineg of $0 ineg
// expect: 2 constructs not supported by the JVM backend
