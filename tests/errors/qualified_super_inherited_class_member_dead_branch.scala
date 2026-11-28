class Parent:
  def count(n: Int): Int = n
trait Numbers extends Parent
class Child extends Parent with Numbers:
  def read(): Int =
    if false then super[Numbers].count(1) else 1

object Probe:
  def main(args: Array[String]): Unit = println(new Child().read())

// A class calls its parent's concrete method through a qualified super selection naming a trait. It
// appears in the untaken branch of a constant conditional.
// scalac 3.8.4 builds it and prints `1`.
// command: build
// teq: --target jvm --all-mains
// jars: scala-library
// exit: 0
