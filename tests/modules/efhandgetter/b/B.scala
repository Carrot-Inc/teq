// A local class's own method named like a getter, no default behind it: scalac prints 42. The readers' contract:
// tests/modules/efpure/a/A.scala.
@main def run(): Unit = println(Lib.test())
