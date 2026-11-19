// expect: b.scala:1:28: error: expected ',' or ')', found identifier
// expect: 1 error found
// absent: a.scala:14:21: error: illegal access to protected
// No record of a phase after the parser's is presented beside a syntax error: scalac reports the other file's syntax
// error alone, and teq checks protected references over the final trees only where the typer reported none
// (the forward-reference check's walk; dotty runs ProtectedAccessors only after a clean typer).
package lib {
  class C(val origin: String) {
    protected def m(x: Int): String = "m" + x + origin
  }
}
package use {
  trait Side { self: lib.C =>
    def f: String = m(1)
  }
}
