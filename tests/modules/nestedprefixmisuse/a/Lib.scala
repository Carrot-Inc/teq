package npma

class O(val n: Int):
  class Item(val value: Int)
  given item: Item = new Item(n)
  class Gen[A](val a: A):
    def get: A = a

object API:
  val a = new O(1)
  val b = new O(2)
  def make: a.Item = new a.Item(7)
  def gen: b.Gen[String] = new b.Gen("g")
