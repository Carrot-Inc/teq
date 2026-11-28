class native extends scala.annotation.StaticAnnotation
class JSGlobal(name: String) extends scala.annotation.StaticAnnotation
@native @JSGlobal("Counter") class Counter:
  def this(n: Int) = this()

object Probe:
  def main(args: Array[String]): Unit = println(new Counter(1) != null)

// A class marked with the program's native and JSGlobal annotations is created with its secondary
// constructor.
// scalac 3.8.4 builds it and prints `true`.
// command: build
// teq: --target jvm --all-mains
// jars: scala-library
// exit: 3
// expect: jvm backend: Probe$.main: a JavaScript class
// expect: 1 construct not supported by the JVM backend
