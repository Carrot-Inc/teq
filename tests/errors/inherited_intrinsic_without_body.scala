class js(code: String) extends scala.annotation.StaticAnnotation
trait Reader:
  @js("5") def read(): Int
class Count extends Reader:
  override def read(): Int = 5
def result(r: Reader): Int = r.read()

object Probe:
  def main(args: Array[String]): Unit = println(result(new Count))

// A trait's abstract method has a JavaScript body, and its implementation returns five.
// scalac 3.8.4 builds it and prints `5`.
// command: build
// teq: --target jvm --all-mains
// jars: scala-library
// exit: 3
// expect: jvm backend: inherited_intrinsic_without_body$package$.result: no @jvm intrinsic or body for read of Reader
// expect: 1 construct not supported by the JVM backend
