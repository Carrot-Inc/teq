class jvm(template: String) extends scala.annotation.StaticAnnotation
class Num(val i: Int)
extension (x: Num = new Num(5)) def +:(y: String): String = y + x.i
@main def run(): Unit = {
  println(+: ("b")())
  println(late())
}
@jvm("bad_late") def fail(): Int = 1
def late(): Int = fail()

// A defaulted right-associative extension receiver precedes a call to a later method.
// scalac 3.8.4 builds it and prints `b5` then `1`.
// The JVM writer fails on a program it should build. These lines pin that failure until it is fixed.
// command: build
// teq: --target jvm --all-mains
// jars: scala-library
// exit: 3
// expect: jvm backend: default_receiver_before_late_method$package$.run: the operand stack model underflowed while writing run()V at line 6
// expect: 1 construct not supported by the JVM backend
