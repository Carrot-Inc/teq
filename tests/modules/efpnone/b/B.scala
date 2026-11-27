// A local class's override without a default over a local trait's with one: scalac prints 12. The readers' contract:
// tests/modules/efpure/a/A.scala.
@main def run(): Unit = println(Lib.none().m(2)())
