class js(code: String) extends scala.annotation.StaticAnnotation
trait Reader:
  @js("5") def read(): Int
class Count extends Reader:
  override def read(): Int = 5
def result(r: Reader): Int =
  extension (x: Int = 1) def +:(y: String): String = y + x
  println(r.read())
  println(+:("b")())
  1

object Probe:
  def main(args: Array[String]): Unit = println(result(new Count))

// The call through the trait precedes a local right-associative extension with a default receiver.
// scalac 3.8.4 builds it and prints `5` then `b1` then `1`.
// command: build
// teq: --target jvm --all-mains
// jars: scala-library
// exit: 3
// expect: jvm backend: method_call_then_default_receiver$package$.result: no @jvm intrinsic or body for read of Reader
// expect: jvm backend: method_call_then_default_receiver$package$.result: the default of a right-associative extension's receiver, whose getter takes the arguments of a clause the call gives after it
// expect: 2 constructs not supported by the JVM backend
