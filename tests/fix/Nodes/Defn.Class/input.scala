/*
rules = ["file:../../support/DumpStructure.scala"]
*/
// Defn.Class, its fields as scalameta 4.17.4's (structure.txt).
package nodes.Defn_Class

object Defn_Class:
  val marker = 0 // assert: DumpStructure.structure
  class A
  final case class B[T](x: T) extends A derives CanEqual
  abstract class C(val y: Int)(using Int) extends A, Serializable
