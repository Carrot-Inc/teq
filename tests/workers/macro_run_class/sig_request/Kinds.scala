package plot

trait Step:
  def apply(n: Int): Int
def make() = new Step { def apply(n: Int): Int = n + 1 }
def topHelper(n: Int): Int = make()(n)
