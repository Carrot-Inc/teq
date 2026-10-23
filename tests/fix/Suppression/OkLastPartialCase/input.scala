/*
rules = ["file:../../support/FlagNumbers.scala"]
*/
// OkLastCase's partial function: a `Term.PartialFunction` is a `Tree.CasesBlock`, its last case's comment the case's.
object O:
  val f: PartialFunction[Int, Int] =
    case 0 => 1 // assert: FlagNumbers.number
    case _ => 2 // scalafix:ok FlagNumbers
  val z = 0
