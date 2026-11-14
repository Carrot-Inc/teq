package ledger

object Use08:
  val first: Int = Tally.lengthOf("alpha08")
  def second: Int = Tally.lengthOf("be08") + first
