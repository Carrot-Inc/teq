def label(): String =
  extension (x: Int = 1) def +:(y: String): String = y + x
  +:("b")()

object Probe:
  def main(args: Array[String]): Unit = println(label())

// A local right-associative extension supplies its default receiver after the explicit argument.
// scalac 3.8.4 builds it and prints `b1`.
// command: build
// teq: --target jvm --all-mains
// jars: scala-library
// exit: 3
// expect: jvm backend: local_extension_default_receiver$package$.label: the default of a right-associative extension's receiver, whose getter takes the arguments of a clause the call gives after it
// expect: 1 construct not supported by the JVM backend
