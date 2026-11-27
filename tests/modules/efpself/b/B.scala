// A default reading the class's own `this`: scalac prints 13. The readers' contract: tests/modules/efpure/a/A.scala.
@main def run(): Unit = println(Lib.self())
