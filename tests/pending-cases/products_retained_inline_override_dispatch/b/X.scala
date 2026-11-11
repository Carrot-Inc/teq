// A downstream of a/ as tests/modules builds it. B's `override inline def m` overrides A's plain `m`, so it is a
// retained inline method (`isRetainedInline`, SymDenotations.scala:1051-1053: inline, not deferred, overriding a
// non-inline member), kept as a method that a call on an `A` reaches by dispatch: scalac 3.8.4 prints the .expected.
// teq's JVM runs, whole and over the products, and its Scala.js and interpreter whole runs print 2. Over teq's products
// of a/, the Scala.js run prints `<function>` and the interpreter's throws StackOverflowError (tests/modules' js-run,
// interp-run). The whole build's classes also differ from the split build's (classes, subset, check-lean-whole,
// bundle). Written with an import: `(s4m.E(): s4m.A).m`, a type selection in an ascription, makes the products build
// panic first ("resolved selection", the writer's missing SELECTtpt arm, area A5's fc9acbf188). Found by reviewer #13
// of the workers exports helper (push 8); the same on integration b602956824.
package s4mu

import s4m.*

@main def run(): Unit =
  val a: A = E()
  println(a.m)
