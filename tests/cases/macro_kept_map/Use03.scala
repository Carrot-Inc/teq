package ledger

object Use03:
  val first: Int = Tally.lengthOf("alpha03")
  def second: Int = Tally.lengthOf("be03") + first
