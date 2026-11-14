package ledger

object Use07:
  val first: Int = Tally.lengthOf("alpha07")
  def second: Int = Tally.lengthOf("be07") + first
