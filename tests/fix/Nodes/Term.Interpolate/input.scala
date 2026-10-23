/*
rules = ["file:../../support/DumpStructure.scala"]
*/
// Term.Interpolate, its fields as scalameta 4.17.4's (structure.txt).
package nodes.Term_Interpolate

object Term_Interpolate:
  val marker = 0 // assert: DumpStructure.structure
  val x = 1
  val a = s"a${x}b$x"
  val b = f"$x%d"
  val c = raw"a\nb"
  val d = s"""t$x
  u"""
  val e = s"$$"
  val f = raw"a$$b\n"
  val g = s"""q$$r
t"""
  val h = s"a\tb$x"
  val i = s"\u0041$x"
  val j = s"$"quote"
