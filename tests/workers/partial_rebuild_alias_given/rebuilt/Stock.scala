package rebuilt

object Shelf:
  val size = 3

object Stock:
  val alias = { import Counter.given; summon[Show[Int]].show(3); Shelf }
