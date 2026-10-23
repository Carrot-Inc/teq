/*
rules = ["file:../../support/DumpStructure.scala"]
*/
// Term.TryWithHandler, its fields as scalameta 4.17.4's (structure.txt).
package nodes.Term_TryWithHandler

object Term_TryWithHandler:
  val marker = 0 // assert: DumpStructure.structure
  val h: PartialFunction[Throwable, Int] = { case _ => 0 }
  def f = try 1 catch h
  def g = try 1 catch h finally println()
