package plot

trait Step:
  def apply(n: Int): Int
def topHelper(n: Int): Int =
  val s = new Step { def apply(n: Int): Int = n + 1 }
  s(n)
