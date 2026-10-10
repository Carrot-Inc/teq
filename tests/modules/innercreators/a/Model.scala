package ia

class O(val n: Int):
  class Inner(val x: Int):
    def get = n + x
trait T:
  val base: Int
  class Box(val v: Int):
    def total = base + v
