// A by-name parameter's default with an effect: scalac prints 32. The readers' contract:
// tests/modules/efpure/a/A.scala.
@main def run(): Unit = println(Lib.byname())
