//> using scala 3.8.4
case class Order(id: Int, total: Int)

extension (o: Order)
  def add(amount: Int): Order = o.copy(total = o.total + amount)
  def add(note: String): String = "order " + o.id + ": " + note
  def add(a: Int, b: Int): Order = o.add(a).add(b)
  def scaled(f: Int => Int): Order = o.copy(total = f(o.total))
  def scaled(by: Int): Order = o.scaled(_ * by)

extension (xs: List[Order])
  def add(amount: Int): List[Order] = xs.map(_.add(amount))

class Basket(val items: List[String]):
  def put(item: String): Basket = Basket(items :+ item)
  def put(item: String, times: Int): Basket = Basket(items ++ List.fill(times)(item))

extension (b: Basket)
  def put(code: Int): Basket = b.put("item#" + code)
  def size: Int = b.items.length

object Syntax:
  extension (s: String)
    def times(n: Int): String = s * n
    def times(n: Int, sep: String): String = List.fill(n)(s).mkString(sep)

import Syntax.*

@main def run(): Unit =
  val o = Order(1, 10)
  println(o.add(5))
  println(o.add("rush"))
  println(o.add(1, 2))
  println(o.scaled(3))
  println(o.scaled(t => t + 1))
  println(List(o).add(1))
  val b = Basket(Nil).put("pen").put("ink", 2).put(7)
  println(b.items)
  println(b.size)
  println("ab".times(2))
  println("ab".times(2, "-"))
