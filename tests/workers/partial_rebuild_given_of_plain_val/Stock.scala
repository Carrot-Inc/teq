package rebuilt

object Stock:
  println("stock-init")
  val before = { import Counter.given; summon[Show[Int]].show(1) }
  val plain = { import Counter.given; summon[Show[Int]].show(3) + "p" }
