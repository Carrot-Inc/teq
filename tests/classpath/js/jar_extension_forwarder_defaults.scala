// jars: scala-library defaults-lib
// targets: js interp
// A jar's forwarders made through a companion extension method (tests/support/defaults_lib.scala's `InlExports` and its
// top-level `tg`), built for JavaScript and run in the interpreter over the jar's converted bodies, every argument
// written: their bodies type as scalac typed them, the parameters carrying no folded default (the forwarders' getters are
// numbered for their members, Namer.scala:1249-1267). Before, the JavaScript build panicked over the jar (reach.rs,
// "resolved selection"), the bodies of `g` and `f` failing teq's typing ("not found: s").
import defaultslib.*

@main def main(): Unit =
  println(InlExports.g("x")(5))
  println(InlExports.f("xyz")(2)(7, 8))
  println(InlExports.k("x")(1, 7, 9))
  println(InlExports.x3("q")(1, "u")(2, 3))
  println(InlExports.ix3("q")(1, "u")(2, 3))
  println("ab".tg(5))
