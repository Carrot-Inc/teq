// A using clause's default reading the first clause: scalac prints 12. The readers' contract:
// tests/modules/efpure/a/A.scala.
@main def run(): Unit = println(Lib.contextual())
