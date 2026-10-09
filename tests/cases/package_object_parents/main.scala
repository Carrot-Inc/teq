package shop

import shop.price._

object Main:
  def main(args: Array[String]): Unit =
    println(implicitly[Show[Double]].show(withTax(10.0)))
    println(price.label("x"))
    println(taxed)
    println(price.showPrice.show(2.5))
    println(2.5.cents)
    println(shop.price.tax)
    println(words.greet("world"))
    println(words.prefix)
    println(After.run())
    println(words.counter.next())
    println(words.counter.next())
    import words.greet
    println(greet("again"))
