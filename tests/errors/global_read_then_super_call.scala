class JSGlobal(name: String) extends scala.annotation.StaticAnnotation
object js:
  def native: String = "sep"
object Paths:
  @JSGlobal("sep") val separator: String = js.native
class jvm(template: String) extends scala.annotation.StaticAnnotation
trait Parent:
  @jvm("iconst_1") def value(): Int = 1
class Child extends Parent:
  def inherited(): Int =
    println(Paths.separator)
    println(super.value())
    1

object Probe:
  def main(args: Array[String]): Unit = println(new Child().inherited())

// The global value selection precedes the call of the parent's annotated method.
// scalac 3.8.4 builds it and prints `sep` then `1` then `1`.
// command: build
// teq: --target jvm --all-mains
// jars: scala-library
// exit: 3
// expect: jvm backend: Child.inherited: JavaScript interop
// expect: jvm backend: Child.inherited: not supported on the JVM yet: a super call of value, which carries its own `@jvm` template
// expect: 2 constructs not supported by the JVM backend
