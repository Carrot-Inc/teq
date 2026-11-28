class native extends scala.annotation.StaticAnnotation
@native trait Counter:
  val count: Int
def read(c: Counter): Int =
  if false then c.count else 1

object Probe:
  def main(args: Array[String]): Unit = println(1)

// The method selects a field of a trait marked with the program's native annotation. The expression
// occurs in the untaken arm of a constant conditional.
// scalac 3.8.4 builds it and prints `1`.
// command: build
// teq: --target jvm --all-mains
// jars: scala-library
// exit: 0
