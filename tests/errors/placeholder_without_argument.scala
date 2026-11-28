class jvm(template: String) extends scala.annotation.StaticAnnotation
@jvm("$1:I") def argument(): Int = 2

object Probe:
  def main(args: Array[String]): Unit = println(argument())

// A parameterless method's template reads a second argument.
// scalac 3.8.4 builds it and prints `2`.
// command: build
// teq: --target jvm --all-mains
// jars: scala-library
// exit: 3
// expect: jvm backend: Probe$.main: the placeholder $1:I of $1:I
// expect: 1 construct not supported by the JVM backend
