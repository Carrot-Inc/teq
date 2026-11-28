class Count(val value: Long) extends AnyVal

abstract class Subject:
  def pick(x: Long): Long
  def pick(x: Count): Long = x.value

class Child extends Subject:
  override def pick(x: Long): Long = x

// The abstract Long overload and the overload taking Count have the same signature after erasure.
// scalac 3.8.4 rejects the conflicting definitions (E120).
// command: build
// teq: --target jvm --all-mains
// jars: scala-library
// exit: 3
// expect: jvm backend: Subject.pick: internal error: a second method pick(J)J in the class
// expect: 1 construct not supported by the JVM backend
