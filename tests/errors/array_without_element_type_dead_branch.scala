class jvm(template: String) extends scala.annotation.StaticAnnotation
@jvm("$0:I array_new") def allocate(n: Int): AnyRef = "array"

object Probe:
  def main(args: Array[String]): Unit =
    println(if false then allocate(1) else 1)

// The template creates an array, but the method declares an AnyRef result. The expression occurs in
// the untaken arm of a constant conditional.
// scalac 3.8.4 builds it and prints `1`.
// command: build
// teq: --target jvm --all-mains
// jars: scala-library
// exit: 0
