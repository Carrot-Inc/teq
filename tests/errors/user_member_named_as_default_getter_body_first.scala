abstract class Subject:
  def pick(x: Int = 42): Int = x
  def `pick$default$1`: Int

class Child extends Subject:
  override def `pick$default$1`: Int = 1

// The abstract method has the name and signature of the getter for pick's default argument.
// scalac 3.8.4 compiles Subject, but the JVM refuses to load it:
// ClassFormatError: Duplicate method name "pick$default$1" with signature "()I" in class file Subject
// command: build
// teq: --target jvm --all-mains
// jars: scala-library
// exit: 3
// expect: jvm backend: Subject.pick$default$1: internal error: a second method pick$default$1()I in the class
// expect: 1 construct not supported by the JVM backend
