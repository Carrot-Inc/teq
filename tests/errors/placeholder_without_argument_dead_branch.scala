class jvm(template: String) extends scala.annotation.StaticAnnotation
@jvm("$1:I") def argument(): Int = 2

object Probe:
  def main(args: Array[String]): Unit =
    println(if false then argument() else 1)

// A parameterless method's template reads a second argument. The expression occurs in the untaken
// arm of a constant conditional.
// scalac 3.8.4 builds it and prints `1`.
// command: build
// teq: --target jvm --all-mains
// jars: scala-library
// exit: 0
