// The second part: its first overload takes three parameters, which no overload of `Reads` does, so a
// comparison that ends at the first difference never reaches the three after it, each of which names a
// val of inferred type.
trait Writes:
  def at(row: Int, col: Int, value: Int): Int
  def at(slot: Stock.second.Slot): Int
  def at(slot: Stock.third.Slot): Int
  def at(slot: Stock.fourth.Slot): Int
