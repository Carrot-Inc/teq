class JSGlobal(name: String) extends scala.annotation.StaticAnnotation
object js:
  def native: String = "sep"
object Paths:
  @JSGlobal("sep") val separator: String = js.native
class Parent:
  def count(n: Int): Int = n
trait Numbers extends Parent
class Child extends Parent with Numbers:
  def read(): Int = super[Numbers].count(Paths.separator.length)

object Probe:
  def main(args: Array[String]): Unit = println(new Child().read())

// A qualified super call takes the length of a JavaScript global as its argument.
// scalac 3.8.4 builds it and prints `3`.
// command: build
// teq: --target jvm --all-mains
// jars: scala-library
// exit: 3
// expect: jvm backend: Child.read: JavaScript interop
// expect: jvm backend: Child.read: not supported on the JVM yet: this super call
// expect: 2 constructs not supported by the JVM backend
