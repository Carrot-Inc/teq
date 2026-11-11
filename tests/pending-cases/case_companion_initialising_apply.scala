// scalac 3.8.4 prints the .expected. teq's interpreter, Scala.js and JVM builds drop `ignore(K2(1))` with the
// initialisation of K2's written companion it runs, and print `|`. dotty elides a case class's `apply` only through
// a synthetic companion (`isCaseClassApply`), or where the application is pure and its qualifier too
// (Inliner.scala:66-82), and K2, written with a val that prints, is an idempotent path, no synthetic one. The same
// on integration a097de53d9 and on master efddb38de3. Found by the interim reviewer of the workers exports helper's
// push 10 (its O1).
case class K2(n: Int)
object K2:
  val z = { print("k2 "); 1 }

inline def ignore(x: Any): Unit = ()

@main def run(): Unit =
  ignore(K2(1)); println("|")
  ignore(new K2(2)); println("|")
