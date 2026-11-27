// A local trait's override's default reached through a second trait and a class, the module's own call: scalac prints
// 12. The readers' contract: tests/modules/efpure/a/A.scala.
@main def run(): Unit = println(Lib.test())
