class jvm(template: String) extends scala.annotation.StaticAnnotation
@jvm("invokevirtual java/lang/String.length()I") def length(): Int = 1

object Probe:
  def main(args: Array[String]): Unit =
    println(if false then length() else 1)

// A parameterless method has an invokevirtual template without a receiver. The expression occurs in
// the untaken arm of a constant conditional.
// scalac 3.8.4 builds it and prints `1`.
// command: build
// teq: --target jvm --all-mains
// jars: scala-library
// exit: 0
