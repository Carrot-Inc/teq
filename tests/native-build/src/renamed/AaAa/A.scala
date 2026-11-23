package renamed

object A {
  def value(x: Int): Int = {
    var n = x
    for (i <- 1 to 10) n = n * 31 + i
    n
  }
}
