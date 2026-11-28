class jvm(template: String) extends scala.annotation.StaticAnnotation
@jvm("$0 ineg") def negate(s: String): Int = 3

object Probe:
  def main(args: Array[String]): Unit = println(negate("x"))

// An integer negation takes a String argument in the template.
// scalac 3.8.4 builds it and prints `3`.
// command: build
// teq: --target jvm --all-mains
// jars: scala-library
// exit: 3
// expect: jvm backend: Probe$.main: the instruction ineg of $0 ineg
// expect: 1 construct not supported by the JVM backend
