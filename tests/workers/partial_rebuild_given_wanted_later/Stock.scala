package rebuilt

object Shelf:
  println("shelf-init")
  val size = 3

object Stock:
  println("stock-init")
  val alias = { import Counter.given; summon[Show[Int]].show(3); Shelf }
