package inc
object Counter:
  def pick(x: Stock.alias.type): String = "alias"
  def pick(x: Stock.second.type): String = "second"
  def pick(x: Any): String = "any"
