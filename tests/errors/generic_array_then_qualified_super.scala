class Parent:
  def count(n: Int): Int = n
trait Numbers extends Parent
class Child extends Parent with Numbers:
  def read[T](zero: T): Int =
    println(scala.newArray[T](1, zero))
    println(super[Numbers].count(1))
    1

object Probe:
  def main(args: Array[String]): Unit = println(new Child().read(1))

// A generic method calls scala.newArray for its type parameter. Next, a class calls its parent's
// concrete method through a qualified super selection naming a trait.
// scalac 3.8.4 rejects it: value newArray is not a member of scala - did you mean scala.Array?
// command: build
// teq: --target jvm --all-mains
// jars: scala-library
// exit: 3
// expect: jvm backend: Child.read: a new array of an element type only a ClassTag knows
// expect: jvm backend: Child.read: not supported on the JVM yet: this super call
// expect: 2 constructs not supported by the JVM backend
