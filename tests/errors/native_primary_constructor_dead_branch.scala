class native extends scala.annotation.StaticAnnotation
class JSGlobal(name: String) extends scala.annotation.StaticAnnotation
@native @JSGlobal("Counter") class Counter

object Probe:
  def main(args: Array[String]): Unit =
    println(if false then new Counter != null else 1)

// A class marked with the program's native and JSGlobal annotations is created with its primary
// constructor. The expression occurs in the untaken arm of a constant conditional.
// scalac 3.8.4 builds it and prints `1`.
// command: build
// teq: --target jvm --all-mains
// jars: scala-library
// exit: 0
