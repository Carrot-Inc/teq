/*
rules = ["file:../../support/Modifiers.scala"]
*/
// An annotation ends where the parser's `annot()` ends it (`ensureApplied(parArgumentExprss(wrapNew(simpleType1())))`):
// a soft modifier after it is the definition's, whatever its type's form, a refinement, a singleton, a tuple (the
// review of `16412b2e9`; the table pass of the fourth fix-up).
package modifiers

import scala.annotation.StaticAnnotation
class ann extends StaticAnnotation
class tagged[T] extends StaticAnnotation
object Mods {
  @ann inline def f = 1 // assert: Modifiers.mod
  @ann transparent inline def g = 2 // assert: Modifiers.mod
  @modifiers.ann() @tagged[Int] inline def h = 3 // assert: Modifiers.mod
  @tagged[{ type T = Int }] inline def i = 4 // assert: Modifiers.mod
  @tagged[Mods.type] inline def j = 5 // assert: Modifiers.mod
  @tagged[(Int, String)] transparent inline def k = 6 // assert: Modifiers.mod
}
