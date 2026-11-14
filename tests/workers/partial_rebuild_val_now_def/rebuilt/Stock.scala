package rebuilt

object Shelf:
  println("shelf-init")
  val size = 3

object Stock:
  println("stock-init")
  def alias = Shelf
