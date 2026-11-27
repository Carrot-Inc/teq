// An overloaded override's default in a local class returned as the interface: scalac prints 22. The readers'
// contract: tests/modules/efpure/a/A.scala.
@main def run(): Unit = println(OE.make().m())
