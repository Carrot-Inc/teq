// An overloaded method's default in a local class: scalac prints 22. The readers' contract:
// tests/modules/efpure/a/A.scala.
@main def run(): Unit = println(Lib.overloaded())
