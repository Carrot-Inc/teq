// A second clause's default reading the first clause's `a` beside the block's own `a`: scalac prints 12. The readers'
// contract: tests/modules/efpure/a/A.scala.
@main def run(): Unit = println(Lib.bind())
