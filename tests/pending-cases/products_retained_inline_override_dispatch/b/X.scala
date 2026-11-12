// A downstream of a/ as tests/modules builds it. B's
// `override inline def m` overrides A's plain `m`, so it is a retained
// inline method (`isRetainedInline`, SymDenotations.scala:1051-1053:
// inline, not deferred, overriding a non-inline member), kept
// as a method that a call on an `A` reaches by dispatch: scalac
// 3.8.4 prints the .expected. Over teq's products of a/, the
// Scala.js and interpreter runs are to print it too (tests/modules'
// js-run, interp-run). The whole build's classes are to be the
// split build's (classes, subset, check-lean-whole, bundle).
package s4mu

import s4m.*

@main def run(): Unit =
  val a: A = E()
  println(a.m)
