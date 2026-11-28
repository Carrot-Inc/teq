class V(val value: Int) extends AnyVal

abstract class Subject:
  def pick(x: V): Int = x.value
  def pick(x: Int): Int

class Child extends Subject:
  override def pick(x: Int): Int = x

// The abstract Int overload and the overload taking V have the same signature after erasure.
// scalac 3.8.4 rejects the conflicting definitions (E120).
// command: build
// teq: --target jvm --all-mains
// jars: scala-library
// exit: 3
// expect: jvm backend: Subject.pick: internal error: a second method pick(I)I in the class
// expect: 1 construct not supported by the JVM backend
