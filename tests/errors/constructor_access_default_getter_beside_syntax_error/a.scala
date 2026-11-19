// expect: b.scala:1:28: error: expected ',' or ')', found identifier
// expect: 1 error found
// absent: a.scala:11:20: error: illegal access to protected
// No record of a phase after the parser's is presented beside a syntax error: scalac reports the other file's syntax
// error alone, and teq checks protected references over the final trees only where the typer reported none
// (the forward-reference check's walk; dotty runs ProtectedAccessors only after a clean typer).
package store {
  class Dflt protected (val a: Int = 1)
}
package shop {
  class D1 extends store.Dflt
}
