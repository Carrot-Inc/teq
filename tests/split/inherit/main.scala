package app

import stock.*
import shop.*

@main def run(): Unit =
  val items: List[Item] = List(Offer("pen", 3), Sale("ink"), Bundle("set", 4), Coupon("X1"))
  items.foreach(i => println(i.describe))
  println(cheapest(items))
  println(items.collect { case o: Offer => o.price }.sum)
