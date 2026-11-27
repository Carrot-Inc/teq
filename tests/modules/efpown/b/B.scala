// A local trait's own override default, the class returned as the interface: scalac prints 12. The readers' contract:
// tests/modules/efpure/a/A.scala.
@main def run(): Unit = println(Lib.own().m(2)())
