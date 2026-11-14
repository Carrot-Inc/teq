object Part10:
  def pad10(n: Int): Int = n + 10
  def run(n: Int): Int =
    def first: Int = second + pad10(n)
    val held: Int = first
    def second: Int = n
    held
