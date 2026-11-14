object Part08:
  def pad8(n: Int): Int = n + 8
  def run(n: Int): Int =
    def first: Int = second + pad8(n)
    val held: Int = first
    def second: Int = n
    held
