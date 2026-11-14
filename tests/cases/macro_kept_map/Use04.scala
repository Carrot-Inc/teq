package ledger

object Use04:
  val first: Int = Tally.lengthOf("alpha04")
  def second: Int = Tally.lengthOf("be04") + first
