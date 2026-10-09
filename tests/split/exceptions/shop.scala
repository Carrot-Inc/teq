package shop

import stock.*

class OutOfStock(val item: String) extends StockError(s"$item is out of stock")

def order(w: Warehouse, item: String, n: Int): String =
  try
    if n == 0 then throw new OutOfStock(item)
    s"$item: ${w.take(n)} left"
  finally
    println(s"ordered $item")
