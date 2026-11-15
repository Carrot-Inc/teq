package inc
object Counter:
  def make(n: Int): Stock.api.Request = new Stock.api.Request(n)
  def id(r: Stock.api.Request): Stock.api.Id = r.n
