class jvm(template: String) extends scala.annotation.StaticAnnotation
@jvm("$0 ineg") def negate(s: String): Int = 3

object Probe:
  def main(args: Array[String]): Unit =
    println(if false then negate("x") else 1)

// An integer negation takes a String argument in the template. The expression occurs in the untaken
// arm of a constant conditional.
// scalac 3.8.4 builds it and prints `1`.
// command: build
// teq: --target jvm --all-mains
// jars: scala-library
// exit: 0
