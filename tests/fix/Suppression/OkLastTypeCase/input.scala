/*
rules = ["file:../../support/FlagNumbers.scala"]
*/
// OkLastCase's match type: a `Type.CasesBlock` is a `Tree.CasesBlock`, its last case's comment the case's.
object O:
  type F[X] = X match
    case Int => 1 // assert: FlagNumbers.number
    case _ => 2 // scalafix:ok FlagNumbers
  val z = 0
