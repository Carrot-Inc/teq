class jvm(template: String) extends scala.annotation.StaticAnnotation
@jvm("$enumValues($0)") def values(): Int = 6

object Probe:
  def main(args: Array[String]): Unit =
    println(if false then values() else 1)

// A parameterless method requests enum values without supplying a companion. The expression occurs
// in the untaken arm of a constant conditional.
// scalac 3.8.4 builds it and prints `1`.
// command: build
// teq: --target jvm --all-mains
// jars: scala-library
// exit: 0
