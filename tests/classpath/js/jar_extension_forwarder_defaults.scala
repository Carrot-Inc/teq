// jars: scala-library defaults-lib
// targets: js interp
// A jar's forwarders made through a companion extension method
// (tests/support/defaults_lib.scala's `InlExports` and its top-level `tg`), built
// for JavaScript and run in the interpreter over the jar's bodies,
// every argument written: their bodies are to type as scalac typed them (the
// forwarders' getters are numbered for their members, Namer.scala:1249-1267).
import defaultslib.*

@main def main(): Unit =
  println(InlExports.g("x")(5))
  println(InlExports.f("xyz")(2)(7, 8))
  println(InlExports.k("x")(1, 7, 9))
  println(InlExports.x3("q")(1, "u")(2, 3))
  println(InlExports.ix3("q")(1, "u")(2, 3))
  println("ab".tg(5))
