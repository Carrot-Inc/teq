class JSGlobal(name: String) extends scala.annotation.StaticAnnotation
object js:
  def native: String = "sep"
object Paths:
  @JSGlobal("sep") val separator: String = js.native

object Probe:
  def main(args: Array[String]): Unit =
    println(if false then Paths.separator else 1)

// A value annotated as a JavaScript global is selected from its owner. The expression occurs in the
// untaken arm of a constant conditional.
// scalac 3.8.4 builds it and prints `1`.
// command: build
// teq: --target jvm --all-mains
// jars: scala-library
// exit: 0
