package ledger

object Use01:
  val first: Int = Tally.lengthOf("alpha01")
  def second: Int = Tally.lengthOf("be01") + first
