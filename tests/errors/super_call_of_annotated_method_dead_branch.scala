class jvm(template: String) extends scala.annotation.StaticAnnotation
trait Parent:
  @jvm("iconst_1") def value(): Int = 1
class Child extends Parent:
  def inherited(): Int =
    if false then super.value() else 1

object Probe:
  def main(args: Array[String]): Unit = println(new Child().inherited())

// A subclass calls the inherited method carrying its own JVM template. The expression occurs in the
// untaken arm of a constant conditional.
// scalac 3.8.4 builds it and prints `1`.
// command: build
// teq: --target jvm --all-mains
// jars: scala-library
// exit: 0
