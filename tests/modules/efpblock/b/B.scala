// A default that is a block with an effect, left twice: scalac prints 8. The readers' contract:
// tests/modules/efpure/a/A.scala.
@main def run(): Unit = println(Lib.block())
