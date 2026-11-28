import scala.annotation.targetName

abstract class Subject:
  @targetName("pick") def choose(x: Int): Int = x
  def pick(x: Int): Int

class Child extends Subject:
  override def pick(x: Int): Int = x

// The target name of choose gives it the same JVM name and signature as the abstract pick.
// scalac 3.8.4 rejects the conflicting definitions (E120).
// command: build
// teq: --target jvm --all-mains
// jars: scala-library
// exit: 3
// expect: jvm backend: Subject.pick: internal error: a second method pick(I)I in the class
// expect: 1 construct not supported by the JVM backend
