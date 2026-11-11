// A downstream of a/ as tests/modules builds it. scalac 3.8.4 prints the .expected, and so does teq's whole build.
// Over teq's products of a/, a run of `Lib`'s bodies (the interpreter over the check build's products; the JavaScript
// over the Scala.js products) calls a/'s top-level overloads by names the products' `Top$package$` does not hold:
// "tov.Top$package$ has no member top$String$Int" (`top$String` with push 13's binary, which also took the wrong
// overload, tests/modules/overloadusing's mechanism). Found beside overloadusing.
package tou

@main def run(): Unit =
  println(tov.Lib.topLiteral)
  println(tov.Lib.topPlain)
