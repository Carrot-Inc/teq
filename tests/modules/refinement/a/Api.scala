package ra

trait Store:
  type Item
  def get: Item

object Stores:
  def ints: Store { type Item = Int } = new Store:
    type Item = Int
    def get: Int = 42
