package ledger

object Use02:
  val first: Int = Tally.lengthOf("alpha02")
  def second: Int = Tally.lengthOf("be02") + first
