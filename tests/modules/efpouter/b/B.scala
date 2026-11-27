// A local class inside a local class, its default reading the enclosing instance: scalac prints 17. The readers'
// contract: tests/modules/efpure/a/A.scala.
@main def run(): Unit = println(Lib.outer())
