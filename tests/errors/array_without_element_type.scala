class jvm(template: String) extends scala.annotation.StaticAnnotation
@jvm("$0:I array_new") def allocate(n: Int): AnyRef = "array"

object Probe:
  def main(args: Array[String]): Unit = println(allocate(1))

// The template creates an array, but the method declares an AnyRef result.
// scalac 3.8.4 builds it and prints `array`.
// command: build
// teq: --target jvm --all-mains
// jars: scala-library
// exit: 3
// expect: jvm backend: Probe$.main: a new array of an element type only a ClassTag knows
// expect: 1 construct not supported by the JVM backend
