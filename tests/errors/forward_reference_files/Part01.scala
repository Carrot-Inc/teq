object Part01:
  def pad1(n: Int): Int = n + 1
  def run(n: Int): Int =
    def first: Int = second + pad1(n)
    val held: Int = first
    def second: Int = n
    held
