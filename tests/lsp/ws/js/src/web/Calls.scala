package web

object Calls:
  def leaf(): Int = 1
  def middle(): Int =
    def local(): Int = leaf() + 1
    val f = (x: Int) => leaf() + x
    local() + f(1)
  def top(): Int = middle() + leaf()
