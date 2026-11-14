package rebuilt

object Counter:
  def take(x: Stock.alias.type): Int = x.size
  val held: Stock.alias.type = Stock.alias
