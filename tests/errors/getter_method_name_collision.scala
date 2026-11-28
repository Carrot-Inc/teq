import scala.annotation.targetName
class Count:
  val x = 1
  @targetName("x") def read(): Int = 2

object Probe:
  def main(args: Array[String]): Unit = println(new Count().read())

// A method's target name and signature coincide with a val's getter.
// scalac 3.8.4 rejects it: Conflicting definitions:
// command: build
// teq: --target jvm --all-mains
// jars: scala-library
// exit: 3
// expect: jvm backend: Count.read: internal error: a second method x()I in the class
// expect: 1 construct not supported by the JVM backend
