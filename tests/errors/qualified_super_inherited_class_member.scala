class Parent:
  def count(n: Int): Int = n
trait Numbers extends Parent
class Child extends Parent with Numbers:
  def read(): Int = super[Numbers].count(1)

object Probe:
  def main(args: Array[String]): Unit = println(new Child().read())

// A class calls its parent's concrete method through a qualified super selection naming a trait.
// scalac 3.8.4 builds it and prints `1`.
// command: build
// teq: --target jvm --all-mains
// jars: scala-library
// exit: 3
// expect: jvm backend: Child.read: not supported on the JVM yet: this super call
// expect: 1 construct not supported by the JVM backend
