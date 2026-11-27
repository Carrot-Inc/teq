// A local class restating the default of the local trait it extends, returned as the interface: scalac prints 13. The
// readers' contract: tests/modules/efpure/a/A.scala.
@main def run(): Unit = println(CO.make().m(2)())
