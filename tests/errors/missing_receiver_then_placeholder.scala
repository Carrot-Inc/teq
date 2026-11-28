class jvm(template: String) extends scala.annotation.StaticAnnotation
@jvm("invokevirtual java/lang/String.length()I") def length(): Int = 1
@jvm("$1:I") def argument(): Int = 2

object Probe:
  def main(args: Array[String]): Unit =
    println(length())
    println(argument())

// The invocation with no receiver precedes the read of a missing argument.
// scalac 3.8.4 builds it and prints `1` then `2`.
// command: build
// teq: --target jvm --all-mains
// jars: scala-library
// exit: 3
// expect: jvm backend: Probe$.main: the instruction invokevirtual of invokevirtual java/lang/String.length()I
// expect: jvm backend: Probe$.main: the placeholder $1:I of $1:I
// expect: 2 constructs not supported by the JVM backend
