package inc
object Shelf:
  println("shelf-init")
  class Tag(val n: Int)
  val size = 3
  def twice(n: Int) = n * 2
object Other
object Stock:
  println("stock-init")
  val alias = Shelf
  val second = Other
