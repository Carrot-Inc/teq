/*
rules = ["file:../../support/DumpStructure.scala"]
*/
// Term.EndMarker, its fields as scalameta 4.17.4's (structure.txt).
package nodes.Term_EndMarker

object Term_EndMarker:
  val marker = 0 // assert: DumpStructure.structure
  def f =
    1
  end f
  val g =
    if true then 1
    else 2
    end if
