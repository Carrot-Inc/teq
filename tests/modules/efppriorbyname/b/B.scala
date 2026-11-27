// A second clause's default reading a by-name first clause: scalac prints 32. The readers' contract:
// tests/modules/efpure/a/A.scala.
@main def run(): Unit = println(Prior.run())
