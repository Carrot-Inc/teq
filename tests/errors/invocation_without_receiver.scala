class jvm(template: String) extends scala.annotation.StaticAnnotation
@jvm("invokevirtual java/lang/String.length()I") def length(): Int = 1

object Probe:
  def main(args: Array[String]): Unit = println(length())

// A parameterless method has an invokevirtual template without a receiver.
// scalac 3.8.4 builds it and prints `1`.
// command: build
// teq: --target jvm --all-mains
// jars: scala-library
// exit: 3
// expect: jvm backend: Probe$.main: the instruction invokevirtual of invokevirtual java/lang/String.length()I
// expect: 1 construct not supported by the JVM backend
