package plot

trait Step:
  def apply(n: Int): Int
def inner(n: Int, s: Step = new Step { def apply(n: Int): Int = n + 1 }): Int = s(n)
def topHelper(n: Int): Int = inner(n)
