package rebuilt

object Inner:
  println("inner-init")
  class Tag(val n: Int)
  val size = 3
  def twice(n: Int) = n * 2

object Stock:
  println("stock-init")
  val alias = Inner
