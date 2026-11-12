package plot

trait Step:
  def apply(n: Int): Int
val stepper = new Step { def apply(n: Int): Int = n + 1 }
def topHelper(n: Int): Int = stepper(n)
