// scalac 3.8.4 prints the .expected. teq's
// interpreter and Scala.js builds are to print it
// too, the unit value held as Any (`println(x)`,
// `println(new M(()).a)`, `println(m.a)`).
class M(val a: Any)
@main def run(): Unit =
  val x: Any = ()
  println(x)
  println(new M(()).a)
  val m = new M(())
  println(m.a)
  println(String.valueOf(m.a))
