package rebuilt

object Shelf:
  val size = 3

object Crate:
  val size = 4

object Stock:
  val first = { import Counter.given; summon[Show[Int]].show(1); Shelf }
  val second = { import Counter.given; summon[Show[Int]].show(2); Crate }
