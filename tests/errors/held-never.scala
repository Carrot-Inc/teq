object L { val b = new StringBuilder; def log(s: String): Unit = { if b.nonEmpty then b ++= ","; b ++= s } }
final case class Num(i: Int)
def num(i: Int): Num = { L.log("n" + i); Num(i) }
def note(s: String): String = { L.log(s); s }
def d(): String = { L.log("d"); "d" }
object S { val n = Num(5); val s = "s5" }
object P { L.log("p"); val n = Num(6) }
object P2 { L.log("p2"); val s = "s7" }
extension (x: Num = num(5)) def +:(y: String): String = y + x.i
@main def run(): Unit =
  val n0 = Num(0)
  val v0 = "v0"
  val res = +:(note("b"))()
  println(res.toString + " " + L.b)
  println(late())

class jvm(template: String) extends scala.annotation.StaticAnnotation
@jvm("bad_late") def fail(): Int = 1
def late(): Int = fail()

// A defaulted receiver precedes the call to the later method.
// scalac 3.8.4 builds it and prints `b5 b,n5` then `1`.
// The JVM writer fails on a program it should build. These lines pin that failure until it is fixed.
// command: build
// teq: --target jvm --all-mains
// jars: scala-library
// exit: 3
// expect: jvm backend: held$minusnever$package$.run: the operand stack model underflowed while writing run()V at line 15
// expect: 1 construct not supported by the JVM backend
