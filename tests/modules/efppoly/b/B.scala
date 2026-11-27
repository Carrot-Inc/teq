// A polymorphic method's default reading the first clause: scalac prints ok. The readers' contract:
// tests/modules/efpure/a/A.scala.
@main def run(): Unit = println(Lib.poly())
