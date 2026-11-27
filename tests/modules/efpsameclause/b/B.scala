// A default in the parameter's own clause reading the block's `a`, not the parameter `a` beside it: scalac prints
// 110. The readers' contract: tests/modules/efpure/a/A.scala.
@main def run(): Unit = println(SC.run())
