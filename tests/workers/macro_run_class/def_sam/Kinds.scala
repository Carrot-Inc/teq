package plot

trait Step:
  def apply(n: Int): Int
def topHelper(n: Int): Int =
  val s: Step = n => n + 1
  s(n)
