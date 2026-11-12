// jars: scala-library defaults-lib
// A jar's inline forwarders made through a companion extension method
// (tests/support/defaults_lib.scala's `InlExports` and its top-level `tg`), called
// and re-exported: each is expanded from its pickled body (`Inlines.inlineCall`
// over the TASTy body, `bodyToInline`), a re-export of one an inline forwarder
// too. The forwarders' getters are numbered for their members (dotty's forwarder
// parameters have no default trees): a call leaving an argument out applies
// the getter of its name, `f`'s `b` the getter of `c`, `h2`'s `b` the inline
// getter of `a`, expanded from its own pickled body, and through a generic
// path method with the call's type arguments (`InlGeneric.r`'s `b`). Q's `g`
// and `h` are to warn as scalac does (E208, the receivers' shifted defaults).
import defaultslib.*

object Q {
  export InlExports.{g, h, f}
}

object R {
  export Q.f
}

@main def main(): Unit =
  println(InlExports.g("x")(5))
  println(InlExports.h("xy")(6, 7))
  println(InlExports.h("xy")(b = 7))
  println(InlExports.h2("x")(a = 5))
  println(InlExports.f("xyz")(2)(7, 8))
  println(InlExports.f("x")(5)(c = 9))
  println(Q.g("x")(5))
  println(Q.h("xy")(6, 7))
  println(R.f("xyz")(2)(7, 8))
  println("ab".tg(5))
  println(InlExports.k("x")(b = 7, c = 9))
  println(InlExports.x3("q")(1, "u")(2, 3))
  println(InlExports.ix3("q")(1, "u")(2, 3))
  println(InlGeneric.r(List(1, 2))(4)(c = 9))
