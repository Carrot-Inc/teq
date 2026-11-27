// A local case class's copy, its defaults the constructor's: scalac prints 11. The readers' contract:
// tests/modules/efpure/a/A.scala.
@main def run(): Unit = println(Lib.copy())
