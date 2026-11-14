package rebuilt

object Counter:
  export Stock.*
  def take(x: Stock.alias.type): Int = x.size
  val held: Stock.alias.type = Stock.alias
