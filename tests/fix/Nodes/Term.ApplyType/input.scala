/*
rules = ["file:../../support/DumpStructure.scala"]
*/
// Term.ApplyType, its fields as scalameta 4.17.4's (structure.txt).
package nodes.Term_ApplyType

object Term_ApplyType:
  val marker = 0 // assert: DumpStructure.structure
  def f[A](a: A) = a
  val x = f[Int](1)
  val y = List.empty[Int]
