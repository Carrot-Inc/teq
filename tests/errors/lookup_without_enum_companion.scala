class jvm(template: String) extends scala.annotation.StaticAnnotation
@jvm("$enumValues($0)") def values(): Int = 6

object Probe:
  def main(args: Array[String]): Unit = println(values())

// A parameterless method requests enum values without supplying a companion.
// scalac 3.8.4 builds it and prints `6`.
// command: build
// teq: --target jvm --all-mains
// jars: scala-library
// exit: 3
// expect: jvm backend: Probe$.main: an enum lookup whose companion is not known
// expect: 1 construct not supported by the JVM backend
