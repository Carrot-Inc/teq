class js(code: String) extends scala.annotation.StaticAnnotation
trait Reader:
  @js("5") def read(): Int
class Count extends Reader:
  override def read(): Int = 5
def result(r: Reader): Int =
  if false then r.read() else 1

object Probe:
  def main(args: Array[String]): Unit = println(result(new Count))

// A trait's abstract method has a JavaScript body, and its implementation returns five. The
// expression occurs in the untaken arm of a constant conditional.
// scalac 3.8.4 builds it and prints `1`.
// command: build
// teq: --target jvm --all-mains
// jars: scala-library
// exit: 0
