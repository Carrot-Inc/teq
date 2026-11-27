// A local class's extension method with a default, called inside the class: scalac prints 9. The readers' contract:
// tests/modules/efpure/a/A.scala.
@main def run(): Unit = println(Lib.extension())
