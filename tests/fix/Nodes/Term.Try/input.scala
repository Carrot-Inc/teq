/*
rules = ["file:../../support/DumpStructure.scala"]
*/
// Term.Try, its fields as scalameta 4.17.4's (structure.txt).
package nodes.Term_Try

object Term_Try:
  val marker = 0 // assert: DumpStructure.structure
  def f = try 1 catch { case e: Exception => 2 } finally println()
  def g = try 1 finally println()
  def h = try 1 catch
    case _: Exception => 2
