// An inline method's local class with a second-clause default, expanded by a middle module's inline method in the
// last module: scalac prints 12. The readers' contract: tests/modules/efpure/a/A.scala.
@main def end(): Unit = println(Mid.run())
