package ledger

object Use05:
  val first: Int = Tally.lengthOf("alpha05")
  def second: Int = Tally.lengthOf("be05") + first
