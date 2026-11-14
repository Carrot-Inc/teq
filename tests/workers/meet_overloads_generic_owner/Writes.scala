trait Writes[U]:
  def at(row: Int, col: Int, value: Int): Int
  def at(slot: Stock.third.Slot): Int
  def at(slots: List[U]): Int
