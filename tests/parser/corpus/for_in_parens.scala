object Shop:
  def total(prices: List[Int],
            discounts: List[Int]): Option[Int] = (for
    p <- prices.headOption
    d <- discounts.headOption
    net = p - d
  yield net).filter(_ > 0)

  def pairs(n: Int): List[(Int, Int)] = List(n).flatMap(k => for
    i <- (1 to k).toList
    j <- (i to k).toList
  yield (i, j))

@main def run(): Unit =
  println(Shop.total(List(10, 20), List(3)))
  println(Shop.total(List(1), List(3)))
  println(Shop.pairs(2))
