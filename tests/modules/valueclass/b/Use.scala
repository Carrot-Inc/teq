package vcb

import vca.*

object Use:
  def main(args: Array[String]): Unit =
    val t = Prices.total(List(new Cents(5), new Cents(7)))
    println(t.show)
    println((t + new Cents(1)).amount)
