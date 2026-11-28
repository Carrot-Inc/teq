class JSGlobal(name: String) extends scala.annotation.StaticAnnotation
object js:
  def native: String = "sep"
object Paths:
  @JSGlobal("sep") val separator: String = js.native

object Probe:
  def main(args: Array[String]): Unit = println(Paths.separator)

// A value annotated as a JavaScript global is selected from its owner.
// scalac 3.8.4 builds it and prints `sep`.
// command: build
// teq: --target jvm --all-mains
// jars: scala-library
// exit: 3
// expect: jvm backend: Probe$.main: JavaScript interop
// expect: 1 construct not supported by the JVM backend
