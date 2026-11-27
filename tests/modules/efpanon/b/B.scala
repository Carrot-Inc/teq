// An anonymous class restating the default of the local trait it extends: scalac prints 13. The readers' contract:
// tests/modules/efpure/a/A.scala.
@main def run(): Unit = println(Lib.anon().m(2)())
