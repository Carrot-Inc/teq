// scalac 3.8.4 prints the .expected. teq is to keep the initialisation of K2's
// written companion that `ignore(K2(1))` runs. dotty elides a case class's
// `apply` only through a synthetic companion (`isCaseClassApply`), or where
// the application is pure and its qualifier too (Inliner.scala:66-82), and K2,
// written with a val that prints, is an idempotent path, no synthetic one. Found
// by the interim reviewer of the workers exports helper's push 10 (its O1).
case class K2(n: Int)
object K2:
  val z = { print("k2 "); 1 }

inline def ignore(x: Any): Unit = ()

@main def run(): Unit =
  ignore(K2(1)); println("|")
  ignore(new K2(2)); println("|")
