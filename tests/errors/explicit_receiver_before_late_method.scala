class jvm(template: String) extends scala.annotation.StaticAnnotation
class Num(val i: Int)
extension (x: Num = new Num(5)) def +:(y: String): String = y + x.i
@main def run(): Unit = {
  println(+: ("b")(new Num(5)))
  println(late())
}
@jvm("bad_late") def fail(): Int = 1
def late(): Int = fail()

// An explicit extension receiver lets the writer reach the later method.
// scalac 3.8.4 builds it and prints `b5` then `1`.
// command: build
// teq: --target jvm --all-mains
// jars: scala-library
// exit: 3
// expect: jvm backend: explicit_receiver_before_late_method$package$.late: the instruction bad_late of bad_late
// expect: 1 construct not supported by the JVM backend
