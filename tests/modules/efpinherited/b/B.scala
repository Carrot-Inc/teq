// A local class inheriting a top-level class's method default: scalac prints 11. The readers' contract:
// tests/modules/efpure/a/A.scala.
@main def run(): Unit = println(Lib.inherited())
