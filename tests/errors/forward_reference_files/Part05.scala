object Part05:
  def pad5(n: Int): Int = n + 5
  def run(n: Int): Int =
    def first: Int = second + pad5(n)
    val held: Int = first
    def second: Int = n
    held
