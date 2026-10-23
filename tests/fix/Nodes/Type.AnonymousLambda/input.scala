/*
rules = ["file:../../support/DumpStructure.scala"]
*/
// Type.AnonymousLambda: a kind projector's type and its placeholders (`-Ykind-projector`), its fields as scalameta 4.17.4's
// (structure.txt).
// teq: --kind-projector
package nodes.Type_AnonymousLambda

object Type_AnonymousLambda:
  val marker = 0 // assert: DumpStructure.structure
  type M = Map[Int, *]
  type E = Either[*, String]
  trait F[G[_]]
  type V = F[Either[+*, Int]]
