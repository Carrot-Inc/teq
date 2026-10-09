// Exception classes spread over two packages: a subclass in `shop` extends a class defined here,
// and `main` catches both, so modules name each other's classes in throws, tests and catches.
package stock

class StockError(message: String) extends Exception(message)

class Warehouse(private var units: Int):
  def take(n: Int): Int =
    if n > units then throw new StockError(s"only $units left")
    units -= n
    units
