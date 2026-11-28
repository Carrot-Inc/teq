def label(): String =
  extension (x: Int = 1) def +:(y: String): String = y + x
  if false then +:("b")() else "end"

object Probe:
  def main(args: Array[String]): Unit = println(label())

// A local right-associative extension supplies its default receiver after the explicit argument. The
// expression occurs in the untaken arm of a constant conditional.
// scalac 3.8.4 builds it and prints `end`.
// command: build
// teq: --target jvm --all-mains
// jars: scala-library
// exit: 0
