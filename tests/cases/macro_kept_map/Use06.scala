package ledger

object Use06:
  val first: Int = Tally.lengthOf("alpha06")
  def second: Int = Tally.lengthOf("be06") + first
