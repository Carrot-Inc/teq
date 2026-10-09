package shop

import stock.*

class Offer(name: String, val price: Int) extends Item(name):
  println(s"Offer $name at $price")

class Sale(name: String) extends Offer(name, 5) with Discounted

case class Coupon(code: String) extends Item("coupon " + code):
  def price = 0

def cheapest(items: List[Item]): Item = items.minBy(_.price)
