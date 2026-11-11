// scalac 3.8.4 prints the .expected, and so does teq's JVM build. teq's interpreter and Scala.js builds print the
// unit value held as Any as `undefined` (`println(x)`, `println(new M(()).a)`, `println(m.a)`);
// `String.valueOf(m.a)` prints `()`. The same on integration a097de53d9 and on master efddb38de3. Found by reviewer
// #15 of the workers exports helper (push 9).
class M(val a: Any)
@main def run(): Unit =
  val x: Any = ()
  println(x)
  println(new M(()).a)
  val m = new M(())
  println(m.a)
  println(String.valueOf(m.a))
