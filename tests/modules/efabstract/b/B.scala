// A local trait restates an interface's method abstract with a default of its own, a local class implements it, the
// module's own call leaves the default: scalac prints 12. The readers' contract: tests/modules/efpure/a/A.scala.
@main def run(): Unit = println(Lib.test())
