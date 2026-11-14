package rebuilt

object Shelf:
  println("shelf-init")
  val size = 3

object Stock:
  println("stock-init")
  val alias = { def shown(n: Int) = { import Counter.given; summon[Show[Int]].show(n) }; shown(3); Shelf }
