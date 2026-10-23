/*
rules = ["file:../../support/FlagNumbers.scala"]
*/
// A `scalafix:ok` after a match's last case suppresses that case alone: its cases block is a `Tree.Block`, which takes
// no trailing comment of a child and passes none to the definition around it (`atPosWithBody`; the pass over
// c511a36d0).
object O:
  def f(x: Int): Int = x match
    case 0 => 1 // assert: FlagNumbers.number
    case _ => 2 // scalafix:ok FlagNumbers
  val z = 0
