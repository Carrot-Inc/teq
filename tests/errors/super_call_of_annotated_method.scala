class jvm(template: String) extends scala.annotation.StaticAnnotation
trait Parent:
  @jvm("iconst_1") def value(): Int = 1
class Child extends Parent:
  def inherited(): Int = super.value()

object Probe:
  def main(args: Array[String]): Unit = println(new Child().inherited())

// A subclass calls the inherited method carrying its own JVM template.
// scalac 3.8.4 builds it and prints `1`.
// command: build
// teq: --target jvm --all-mains
// jars: scala-library
// exit: 3
// expect: jvm backend: Child.inherited: not supported on the JVM yet: a super call of value, which carries its own `@jvm` template
// expect: 1 construct not supported by the JVM backend
