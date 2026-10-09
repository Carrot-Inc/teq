package xpa.inner

class Thing(val n: Int)
object Thing:
  def one: Thing = new Thing(1)

def helper(x: Int): Int = x + 1
val limit: Int = 3
type Pairs = List[(Int, Int)]

extension (i: Int) def twice: Int = i * 2

object Tools:
  def hammer: String = "hammer"
